//! Package registry module

mod packagist;

pub use packagist::{PackageDist, PackageMetadata, PackageSource, PackagistClient, VersionMetadata};
