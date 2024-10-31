use osmquadtree::elements::{Block, Quadtree, WithQuadtree};
use crate::elements::GeoJsonable;
use crate::pack_geometry::{pack_geometry_block, unpack_geometry_block};
use crate::{
    ComplicatedPolygonGeometry, LinestringGeometry, PointGeometry, SimplePolygonGeometry, Result
};

use osmquadtree::utils::timestamp_string;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::fmt;



#[derive(Debug,Serialize)]
pub enum GeometryElement {
    Point(PointGeometry),
    Linestring(LinestringGeometry),
    SimplePolygon(SimplePolygonGeometry),
    ComplicatedPolygon(ComplicatedPolygonGeometry)
}

impl WithQuadtree for GeometryElement {
    fn get_quadtree<'a>(&'a self) -> &'a Quadtree {
        match self {
            GeometryElement::Point(p) => &p.quadtree,
            GeometryElement::Linestring(p) => &p.quadtree,
            GeometryElement::SimplePolygon(p) => &p.quadtree,
            GeometryElement::ComplicatedPolygon(p) => &p.quadtree,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GeometryBlock {
    pub index: i64,
    pub quadtree: Quadtree,
    pub end_date: i64,

    pub points: Vec<PointGeometry>,
    pub linestrings: Vec<LinestringGeometry>,
    pub simple_polygons: Vec<SimplePolygonGeometry>,
    pub complicated_polygons: Vec<ComplicatedPolygonGeometry>,
}

impl Block for GeometryBlock {
    type Element=GeometryElement;
    fn with_quadtree(q: Quadtree) -> Self {
        GeometryBlock::new(0, q, 0)
    }

    fn get_index(&self) -> i64 {
        self.index
    }
    fn get_quadtree<'a>(&'a self) -> &'a Quadtree {
        &self.quadtree
    }
    fn get_end_date(&self) -> i64 {
        self.end_date
    }

    fn len(&self) -> usize {
        self.points.len()
            + self.linestrings.len()
            + self.simple_polygons.len()
            + self.complicated_polygons.len()
    }
    fn weight(&self) -> usize {
        self.points.len()
            + 8 * self.linestrings.len()
            + 8 * self.simple_polygons.len()
            + 20 * self.complicated_polygons.len()
    }
    
    fn add_object(&mut self, p: Self::Element) -> osmquadtree::Result<()> {
        match p {
            GeometryElement::Point(p) => {
                self.points.push(p);
                Ok(())
            }
            GeometryElement::Linestring(l) => {
                self.linestrings.push(l);
                Ok(())
            }
            GeometryElement::SimplePolygon(sp) => {
                self.simple_polygons.push(sp);
                Ok(())
            }
            GeometryElement::ComplicatedPolygon(cp) => {
                self.complicated_polygons.push(cp);
                Ok(())
            }
            
        }
    }
    

    fn sort(&mut self) {
        self.points.sort_by_key(|p| p.id);
        self.linestrings.sort_by_key(|p| p.id);
        self.simple_polygons.sort_by_key(|p| p.id);
        self.complicated_polygons.sort_by_key(|p| p.id);
    }
}

impl From<PointGeometry> for GeometryElement {
    fn from(n: PointGeometry) -> GeometryElement {
        GeometryElement::Point(n)
    }
}
impl From<LinestringGeometry> for GeometryElement {
    fn from(n: LinestringGeometry) -> GeometryElement {
        GeometryElement::Linestring(n)
    }
}
impl From<SimplePolygonGeometry> for GeometryElement {
    fn from(n: SimplePolygonGeometry) -> GeometryElement {
        GeometryElement::SimplePolygon(n)
    }
}

impl From<ComplicatedPolygonGeometry> for GeometryElement {
    fn from(n: ComplicatedPolygonGeometry) -> GeometryElement {
        GeometryElement::ComplicatedPolygon(n)
    }
}

impl IntoIterator for GeometryBlock {
    type Item = GeometryElement;
    type IntoIter = Box<dyn Iterator<Item = GeometryElement>>;
    fn into_iter(self: Self) -> Self::IntoIter {
        Box::new(
            self.points
                .into_iter()
                .map(GeometryElement::from)
                .chain(self.linestrings.into_iter().map(GeometryElement::from))
                .chain(self.simple_polygons.into_iter().map(GeometryElement::from))
                .chain(self.complicated_polygons.into_iter().map(GeometryElement::from))
        )
    }
}

impl GeometryBlock {
    pub fn new(index: i64, quadtree: Quadtree, end_date: i64) -> GeometryBlock {
        GeometryBlock {
            index: index,
            quadtree: quadtree,
            end_date: end_date,
            points: Vec::new(),
            linestrings: Vec::new(),
            simple_polygons: Vec::new(),
            complicated_polygons: Vec::new(),
        }
    }
    
    pub fn unpack(index: i64, data: &[u8]) -> Result<GeometryBlock> {
        unpack_geometry_block(index, data)
    }

    pub fn pack(&self) -> Result<Vec<u8>> {
        pack_geometry_block(self)
    }
    

    pub fn extend(&mut self, other: GeometryBlock) {
        self.points.extend(other.points);
        self.linestrings.extend(other.linestrings);
        self.simple_polygons.extend(other.simple_polygons);
        self.complicated_polygons.extend(other.complicated_polygons);
    }
    
    pub fn extend_clone(&mut self, other: &GeometryBlock) {
        self.points.extend(other.points.clone());
        self.linestrings.extend(other.linestrings.clone());
        self.simple_polygons.extend(other.simple_polygons.clone());
        self.complicated_polygons.extend(other.complicated_polygons.clone());
    }

    pub fn to_geojson(&self, transform: bool) -> Result<Value> {
        let mut rr = Map::new();

        rr.insert(
            String::from("quadtree"),
            if self.quadtree.is_empty() {
                json!(())
            } else {
                json!(self.quadtree.as_tuple().xyz())
            },
        );
        rr.insert(
            String::from("end_date"),
            json!(timestamp_string(self.end_date)),
        );
        let mut points = Vec::new();
        for p in &self.points {
            points.push(p.to_geojson(transform)?);
        }
        rr.insert(String::from("points"), json!(points));

        let mut linestrings = Vec::new();
        for p in &self.linestrings {
            linestrings.push(p.to_geojson(transform)?);
        }
        rr.insert(String::from("linestrings"), json!(linestrings));

        let mut simple_polygons = Vec::new();
        for p in &self.simple_polygons {
            simple_polygons.push(p.to_geojson(transform)?);
        }
        rr.insert(String::from("simple_polygons"), json!(simple_polygons));

        let mut complicated_polygons = Vec::new();
        for p in &self.complicated_polygons {
            complicated_polygons.push(p.to_geojson(transform)?);
        }
        rr.insert(
            String::from("complicated_polygons"),
            json!(complicated_polygons),
        );

        Ok(json!(rr))
    }
}
impl fmt::Display for GeometryBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GeometryBlock[{} [{}] with {} points, {} linestrings, {} simple polygons, {} complicated polgons]", 
            self.index,  self.quadtree,
            self.points.len(), self.linestrings.len(),
            self.simple_polygons.len(), self.complicated_polygons.len())
    }
}
