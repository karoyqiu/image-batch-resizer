pub mod resize;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use rayon::prelude::*;
use tauri::{Emitter, State};

#[derive(Clone, serde::Serialize)]
struct Progress {
  done: usize,
  total: usize,
}

#[derive(Clone, serde::Serialize)]
struct Summary {
  succeeded: usize,
  failed: usize,
  skipped: usize,
}

#[derive(Default)]
struct CancelFlag(Arc<AtomicBool>);

/// Pre-flight the plan, then run the batch on a background thread: each source
/// decodes once, every rule is applied, failures are skipped, and Stop cancels
/// mid-run (already-written outputs are kept). Emits `progress` per item and
/// `finished` with a summary.
#[tauri::command]
fn start_batch(
  app: tauri::AppHandle,
  state: State<'_, CancelFlag>,
  sources: Vec<PathBuf>,
  rules: Vec<resize::ResizeRule>,
  dest: PathBuf,
) -> Result<(), String> {
  let plan = resize::plan(&sources, &rules, &dest);
  if let Err(clashes) = resize::check_collisions(&plan, &sources) {
    return Err(clashes.join("\n"));
  }

  let cancel = state.0.clone();
  cancel.store(false, Ordering::SeqCst);

  let total = plan.len();
  std::thread::spawn(move || {
    let done = AtomicUsize::new(0);
    let succeeded = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);

    // Group the (source-major) plan by source so each source decodes once.
    let mut by_source: Vec<(PathBuf, Vec<resize::OutputItem>)> = Vec::new();
    for item in plan {
      match by_source.last_mut() {
        Some((src, items)) if *src == item.source => items.push(item),
        _ => by_source.push((item.source.clone(), vec![item])),
      }
    }

    by_source.par_iter().for_each(|(source, items)| {
      if cancel.load(Ordering::SeqCst) {
        return;
      }
      // One progress tick per finished item, whether it succeeded or failed.
      let tick = || {
        let done = done.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = app.emit("progress", Progress { done, total });
      };
      let img = match image::open(source) {
        Ok(img) => img,
        Err(_) => {
          failed.fetch_add(items.len(), Ordering::Relaxed);
          for _ in items {
            tick();
          }
          return;
        }
      };
      for item in items {
        if cancel.load(Ordering::SeqCst) {
          break;
        }
        let ok = match resize::resize_image(&img, &item.rule) {
          Ok(bytes) => std::fs::write(&item.out_path, bytes).is_ok(),
          Err(_) => false,
        };
        if ok {
          succeeded.fetch_add(1, Ordering::Relaxed);
        } else {
          failed.fetch_add(1, Ordering::Relaxed);
        }
        tick();
      }
    });

    let succeeded = succeeded.load(Ordering::Relaxed);
    let failed = failed.load(Ordering::Relaxed);
    let _ = app.emit(
      "finished",
      Summary {
        succeeded,
        failed,
        skipped: total.saturating_sub(succeeded + failed),
      },
    );
  });

  Ok(())
}

#[tauri::command]
fn stop_batch(state: State<'_, CancelFlag>) {
  state.0.store(true, Ordering::SeqCst);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .manage(CancelFlag::default())
    .invoke_handler(tauri::generate_handler![start_batch, stop_batch])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
