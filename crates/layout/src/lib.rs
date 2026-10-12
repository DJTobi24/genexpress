//! Layout for GenHa (layer L2): pure functions over the document model that arrange things.
//!
//! - [`smart_resize`]: resize a page to another size while keeping the design readable;
//! - [`align`], [`distribute`], [`tidy_grid`]: arrange a selection;
//! - [`collage_layouts`] / [`cells_in`]: photo grids for 1 to 9 pictures;
//! - [`snap_guides`] / [`snap_frame`]: smart guides while dragging.
//!
//! Nothing here mutates a document; callers apply the returned frames or page through the engine
//! (so every change is undoable).
#![forbid(unsafe_code)]

mod arrange;
mod collage;
mod resize;
mod snap;

pub use arrange::{Align, Axis, align, distribute, selection_bounds, tidy_grid};
pub use collage::{CollageLayout, MAX_COLLAGE, cells_in, collage_layouts};
pub use resize::smart_resize;
pub use snap::{SnapGuides, snap_frame, snap_guides};
