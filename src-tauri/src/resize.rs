use std::path::{Path, PathBuf};

use image::{imageops::FilterType, DynamicImage, ImageFormat, ImageResult, Rgb, RgbImage, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
  Png,
  Jpg,
}

impl Format {
  pub fn ext(self) -> &'static str {
    match self {
      Format::Png => "png",
      Format::Jpg => "jpg",
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeRule {
  pub width: u32,
  pub height: u32,
  pub format: Format,
  pub suffix: String,
}

/// Derive an output path: `{dest}/{stem}{suffix}.{ext}`.
/// `stem` is the source filename stem (everything before the final dot),
/// and `ext` is the rule format's lowercase extension.
pub fn output_path(source: &Path, rule: &ResizeRule, dest: &Path) -> PathBuf {
  let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("");
  let mut name = String::with_capacity(stem.len() + rule.suffix.len() + 4);
  name.push_str(stem);
  name.push_str(&rule.suffix);
  name.push('.');
  name.push_str(rule.format.ext());
  dest.join(name)
}

#[derive(Debug, Clone)]
pub struct OutputItem {
  pub source: PathBuf,
  pub out_path: PathBuf,
  pub rule: ResizeRule,
}

/// Build the full work list: every rule applied to every source (`N * M` items),
/// source-major (for each source, every rule in order).
pub fn plan(sources: &[PathBuf], rules: &[ResizeRule], dest: &Path) -> Vec<OutputItem> {
  let mut items = Vec::with_capacity(sources.len().saturating_mul(rules.len()));
  for source in sources {
    for rule in rules {
      let out_path = output_path(source, rule, dest);
      items.push(OutputItem {
        source: source.clone(),
        out_path,
        rule: rule.clone(),
      });
    }
  }
  items
}

/// Pre-flight collision check (ADR-0001). Rejects when two outputs resolve to the
/// same path, or when an output would overwrite a selected source file. Prior-run
/// outputs already on disk are intentionally not checked — overwriting those is the
/// intended re-run behavior. Returns Err listing every clash found.
pub fn check_collisions(plan: &[OutputItem], sources: &[PathBuf]) -> Result<(), Vec<String>> {
  use std::collections::{HashMap, HashSet};

  let mut errors: Vec<String> = Vec::new();

  // Intra-batch: group by out_path; any path with >1 item is a clash.
  let mut by_path: HashMap<&Path, Vec<&OutputItem>> = HashMap::new();
  for item in plan {
    by_path.entry(item.out_path.as_path()).or_default().push(item);
  }
  for (path, items) in by_path {
    if items.len() > 1 {
      let origins: Vec<String> = items.iter().map(|i| i.source.display().to_string()).collect();
      errors.push(format!(
        "multiple outputs write to {}: from {}",
        path.display(),
        origins.join(", ")
      ));
    }
  }

  // Source guard: no output may overwrite a selected source file.
  let source_set: HashSet<&Path> = sources.iter().map(PathBuf::as_path).collect();
  for item in plan {
    if source_set.contains(item.out_path.as_path()) {
      errors.push(format!("output would overwrite source file: {}", item.out_path.display()));
    }
  }

  if errors.is_empty() {
    Ok(())
  } else {
    Err(errors)
  }
}

/// Resize `img` to the rule's exact dimensions at best quality (Lanczos3), then
/// encode. JPG flattens transparency onto white; PNG passes alpha through.
pub fn resize_image(img: &DynamicImage, rule: &ResizeRule) -> ImageResult<Vec<u8>> {
  let resized = img.resize_exact(rule.width, rule.height, FilterType::Lanczos3);
  let mut buf = std::io::Cursor::new(Vec::new());
  match rule.format {
    Format::Png => resized.write_to(&mut buf, ImageFormat::Png)?,
    Format::Jpg => {
      let rgb = flatten_on_white(&resized.to_rgba8());
      image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90)
        .encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)?;
    }
  }
  Ok(buf.into_inner())
}

/// Composite an RGBA image onto an opaque white RGB background.
fn flatten_on_white(rgba: &RgbaImage) -> RgbImage {
  let (width, height) = rgba.dimensions();
  let mut out = RgbImage::new(width, height);
  for y in 0..height {
    for x in 0..width {
      let Rgba([r, g, b, a]) = rgba.get_pixel(x, y);
      let alpha = *a as f32 / 255.0;
      let blend = |channel: u8| -> u8 {
        ((channel as f32 * alpha) + (255.0 * (1.0 - alpha))).round() as u8
      };
      out.put_pixel(x, y, Rgb([blend(*r), blend(*g), blend(*b)]));
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use image::GenericImageView;

  #[test]
  fn derives_output_filename_from_stem_suffix_and_format() {
    let source = Path::new("photos/photo.png");
    let rule = ResizeRule {
      width: 100,
      height: 100,
      format: Format::Png,
      suffix: "_thumb".into(),
    };
    let dest = Path::new("/out");

    assert_eq!(output_path(&source, &rule, dest), dest.join("photo_thumb.png"));
  }

  #[test]
  fn stem_keeps_extra_dots_only_the_final_extension_is_split() {
    // `my.photo.png` -> stem `my.photo`, not `my`
    let source = Path::new("my.photo.png");
    let rule = ResizeRule {
      width: 1,
      height: 1,
      format: Format::Png,
      suffix: "_x".into(),
    };
    let dest = Path::new("/out");

    assert_eq!(output_path(&source, &rule, dest), dest.join("my.photo_x.png"));
  }

  #[test]
  fn output_extension_is_lowercase_from_the_rule_format() {
    // Uppercase source extension does not leak; output ext comes from the rule.
    let source = Path::new("PHOTO.PNG");
    let rule = ResizeRule {
      width: 1,
      height: 1,
      format: Format::Jpg,
      suffix: "_x".into(),
    };
    let dest = Path::new("/out");

    assert_eq!(output_path(&source, &rule, dest), dest.join("PHOTO_x.jpg"));
  }

  #[test]
  fn plan_is_every_rule_applied_to_every_source() {
    let sources = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
    let rule = |suffix: &str| ResizeRule {
      width: 10,
      height: 10,
      format: Format::Png,
      suffix: suffix.into(),
    };
    let rules = vec![rule("_1"), rule("_2")];
    let dest = Path::new("/out");

    let plan = plan(&sources, &rules, dest);

    assert_eq!(plan.len(), 4);
    // source-major order: a_1, a_2, b_1, b_2
    assert_eq!(plan[0].out_path, dest.join("a_1.png"));
    assert_eq!(plan[1].out_path, dest.join("a_2.png"));
    assert_eq!(plan[2].out_path, dest.join("b_1.png"));
    assert_eq!(plan[3].out_path, dest.join("b_2.png"));
    assert_eq!(plan[0].source, PathBuf::from("a.png"));
    assert_eq!(plan[0].rule.suffix, "_1");
  }

  #[test]
  fn plan_is_empty_when_no_sources_or_no_rules() {
    let dest = Path::new("/out");
    let rule = ResizeRule {
      width: 1,
      height: 1,
      format: Format::Png,
      suffix: "_".into(),
    };
    assert!(plan(&[], &[rule.clone()], dest).is_empty());
    assert!(plan(&[PathBuf::from("a.png")], &[], dest).is_empty());
  }

  #[test]
  fn collision_when_two_outputs_share_a_path() {
    // a.png and a.jpg with the same suffix+format both map to a_x.png
    let sources = vec![PathBuf::from("a.png"), PathBuf::from("a.jpg")];
    let rule = ResizeRule {
      width: 1,
      height: 1,
      format: Format::Png,
      suffix: "_x".into(),
    };
    let dest = Path::new("/out");
    let plan = plan(&sources, &[rule], dest);

    assert!(check_collisions(&plan, &sources).is_err());
  }

  #[test]
  fn collision_when_output_overwrites_a_source_file() {
    // empty suffix into the source dir: out_path == source
    let sources = vec![PathBuf::from("/src/a.png")];
    let rule = ResizeRule {
      width: 1,
      height: 1,
      format: Format::Png,
      suffix: String::new(),
    };
    let dest = Path::new("/src");
    let plan = plan(&sources, &[rule], dest);

    assert!(check_collisions(&plan, &sources).is_err());
  }

  #[test]
  fn no_collision_for_distinct_outputs() {
    let sources = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
    let rules = vec![
      ResizeRule {
        width: 1,
        height: 1,
        format: Format::Png,
        suffix: "_1".into(),
      },
      ResizeRule {
        width: 2,
        height: 2,
        format: Format::Jpg,
        suffix: "_2".into(),
      },
    ];
    let dest = Path::new("/out");
    let plan = plan(&sources, &rules, dest);

    assert!(check_collisions(&plan, &sources).is_ok());
  }

  fn uniform_transparent(w: u32, h: u32) -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0])))
  }

  fn uniform_opaque(w: u32, h: u32) -> DynamicImage {
    DynamicImage::ImageRgb8(RgbImage::from_pixel(w, h, Rgb([255, 0, 0])))
  }

  #[test]
  fn resize_hits_exact_target_dimensions() {
    let img = uniform_opaque(4, 4);
    let rule = ResizeRule {
      width: 2,
      height: 3,
      format: Format::Png,
      suffix: "_".into(),
    };
    let bytes = resize_image(&img, &rule).unwrap();
    let out = image::load_from_memory(&bytes).unwrap();

    assert_eq!(out.dimensions(), (2, 3));
  }

  #[test]
  fn png_preserves_alpha_for_transparent_source() {
    let img = uniform_transparent(4, 4);
    let rule = ResizeRule {
      width: 2,
      height: 2,
      format: Format::Png,
      suffix: "_".into(),
    };
    let bytes = resize_image(&img, &rule).unwrap();
    let out = image::load_from_memory(&bytes).unwrap();

    assert_eq!(out.color(), image::ColorType::Rgba8);
    assert_eq!(out.to_rgba8().get_pixel(0, 0), &Rgba([0, 0, 0, 0]));
  }

  #[test]
  fn png_does_not_fabricate_alpha_for_opaque_source() {
    let img = uniform_opaque(4, 4);
    let rule = ResizeRule {
      width: 2,
      height: 2,
      format: Format::Png,
      suffix: "_".into(),
    };
    let bytes = resize_image(&img, &rule).unwrap();
    let out = image::load_from_memory(&bytes).unwrap();

    assert_eq!(out.color(), image::ColorType::Rgb8);
  }

  #[test]
  fn jpg_flattens_transparency_onto_white() {
    let img = uniform_transparent(4, 4);
    let rule = ResizeRule {
      width: 2,
      height: 2,
      format: Format::Jpg,
      suffix: "_".into(),
    };
    let bytes = resize_image(&img, &rule).unwrap();
    let out = image::load_from_memory(&bytes).unwrap();

    assert_eq!(out.color(), image::ColorType::Rgb8);
    let [r, g, b] = out.to_rgb8().get_pixel(0, 0).0;
    assert!([r, g, b].iter().all(|c| *c >= 250), "expected white, got {r},{g},{b}");
  }
}
