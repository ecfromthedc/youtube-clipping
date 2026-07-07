//! One module per page — page agents own exactly one file each and never edit
//! shared files (main.rs, http.rs, this mod list, styles.css).

pub mod analytics;
pub mod dashboard;
pub mod new_project;
pub mod pipeline;
pub mod project;
pub mod studio;
pub mod studio_format;
