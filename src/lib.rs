mod addparenttag;
mod default_minzoom_values;
mod default_style;
mod elements;
mod geometry_block;
mod minzoom;
mod multipolygons;
mod pack_geometry;
mod position;
pub mod postgresql;
mod process_geometry;
mod relationtags;
mod style;
mod waywithnodes;
mod wkb;
mod tempfile;



use osmquadtree::elements::{Element, Node, Quadtree, Relation, Way};
use osmquadtree::sortblocks::TempData;
pub use crate::position::{get_srid, LonLat, XY,calc_line_length};
pub use crate::waywithnodes::{CollectWayNodes,Locations};

pub use crate::elements::{
    ComplicatedPolygonGeometry, LinestringGeometry, PointGeometry, PolygonPart, Ring, RingPart,
    SimplePolygonGeometry, GeoJsonable, WithBounds
};
pub use crate::geometry_block::{GeometryElement,GeometryBlock};
pub use crate::process_geometry::{process_geometry, OutputType, StoreBlocks,process_geometry_call};
pub use crate::style::GeometryStyle;
pub use crate::tempfile::{prep_write_geometry_pbffile, make_write_temp_geometry, write_temp_geometry};
pub use crate::minzoom::{FindMinZoom, MinZoomSpec};
pub use crate::default_minzoom_values::DEFAULT_MINZOOM_VALUES;

pub use pack_geometry::read_geometry_blocks;


pub use osmquadtree::{Error,Result};

use std::collections::BTreeMap;

pub struct WorkingBlock {
    pub geometry_block: GeometryBlock,

    pub pending_nodes: Vec<Node>,
    pub pending_ways: Vec<(Way, Vec<LonLat>)>,
    pub pending_relations: Vec<Relation>,
}
impl WorkingBlock {
    pub fn new(index: i64, quadtree: Quadtree, end_date: i64) -> WorkingBlock {
        WorkingBlock {
            geometry_block: GeometryBlock::new(index, quadtree, end_date),
            pending_nodes: Vec::new(),
            pending_ways: Vec::new(),
            pending_relations: Vec::new(),
        }
    }
}

pub enum OtherData {
    Errors(Vec<(Element, String)>),
    Messages(Vec<String>),
    GeometryBlocks(BTreeMap<Quadtree, GeometryBlock>),
    TempData(TempData),
}

pub type Timings = channelled_callbacks::Timings<OtherData>;

pub type CallFinishGeometryBlock =
    Box<dyn channelled_callbacks::CallFinish<CallType = GeometryBlock, ReturnType = Timings, ErrorType=Error>>;
