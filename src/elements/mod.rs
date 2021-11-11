mod complicatedpolygongeometry;
mod linestringgeometry;
mod pointgeometry;
mod simplepolygongeometry;

pub use complicatedpolygongeometry::{
    collect_rings, ComplicatedPolygonGeometry, PolygonPart, Ring, RingPart,
};
pub use linestringgeometry::LinestringGeometry;
pub use pointgeometry::PointGeometry;
pub use simplepolygongeometry::SimplePolygonGeometry;

use osmquadtree::elements::Bbox;

pub trait GeoJsonable {
    fn to_geojson(&self, transform: bool) -> std::io::Result<serde_json::Value>;
}

pub trait WithBounds {
    fn bounds(&self) -> Bbox;
}
