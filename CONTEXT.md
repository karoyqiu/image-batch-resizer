# Image Batch Resizer

A desktop tool that turns a set of source images into a larger set of resized images, by applying every resize rule to every source.

## Language

### Inputs

**Source File**:
A single PNG or JPEG image selected by the user as input.
_Avoid_: input image, original, source image

**Resize Rule**:
A fixed output size and format to apply to every source file. Carries a **Width** and **Height** in pixels (both always required, the image stretched to that exact size — aspect ratio ignored, upscaling allowed), a **Format** (PNG or JPG), and a **Suffix** appended to the output filename.
_Avoid_: preset, profile, setting, filter

### Outputs

**Output File**:
One resized image produced by applying one resize rule to one source file. For `N` sources and `M` rules there are `N × M` output files.
_Avoid_: result, generated file, export

**Format**:
The encoding of an output file. Either PNG or JPG.
_Avoid_: type, extension, filetype

**Suffix**:
A string on a resize rule inserted between the source filename stem and the output extension to form the output filename (`{stem}{suffix}.{ext}`).
_Avoid_: tag, marker, postfix
