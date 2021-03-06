use osmquadtree::elements::{coordinate_as_float, Bbox, Info, Quadtree, Tag, Way};

use crate::elements::pointgeometry::pack_tags;
use crate::elements::{GeoJsonable,WithBounds};
use crate::wkb::{prep_wkb, write_ring, write_uint32};
use crate::LonLat;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::borrow::Borrow;
//extern crate geo;

pub fn read_lonlats<T: Borrow<LonLat>>(lonlats: &Vec<T>, is_reversed: bool, transform: bool) -> Vec<(f64, f64)> {
    let mut res = Vec::with_capacity(lonlats.len());
    for l in lonlats {
        let p = l.borrow(); //.forward();
        if transform {
            let q = p.forward();
            res.push((q.x, q.y));
        } else {
            res.push((coordinate_as_float(p.lon), coordinate_as_float(p.lat)));
        }
    }
    if is_reversed {
        res.reverse();
    }
    res
}
pub fn pack_bounds(bounds: &Bbox, transform: bool) -> Value {
    
    if transform {
        let a = LonLat::new(bounds.minlon, bounds.minlat).forward();
        let b = LonLat::new(bounds.maxlon, bounds.maxlat).forward();
        json!((a.x,a.y,b.x,b.y))
    } else {
        json!((
            coordinate_as_float(bounds.minlon),
            coordinate_as_float(bounds.minlat),
            coordinate_as_float(bounds.maxlon),
            coordinate_as_float(bounds.maxlat)
        ))
    }
}

#[derive(Debug, Serialize,Clone)]
pub struct SimplePolygonGeometry {
    pub id: i64,
    pub info: Option<Info>,
    pub tags: Vec<Tag>,
    pub refs: Vec<i64>,
    pub lonlats: Vec<LonLat>,
    pub area: f64,
    pub reversed: bool,
    pub z_order: Option<i64>,
    pub layer: Option<i64>,
    pub minzoom: Option<i64>,
    pub quadtree: Quadtree,
}
impl WithBounds for SimplePolygonGeometry {
    fn bounds(&self) -> Bbox {
        let mut res = Bbox::empty();
        for l in &self.lonlats {
            res.expand(l.lon, l.lat);
        }
        res
    }
}
impl SimplePolygonGeometry {
    pub fn empty() -> SimplePolygonGeometry {
        SimplePolygonGeometry{id: 0, info: None, tags: Vec::new(), refs: Vec::new(), lonlats: Vec::new(),
            area: 0.0, reversed: false, layer: None, z_order: None, minzoom: None, quadtree: Quadtree::empty()}
    }
    
    
    pub fn from_way(
        w: Way,
        lonlats: Vec<LonLat>,
        tgs: Vec<Tag>,
        area: f64,
        layer: Option<i64>,
        z_order: Option<i64>,
        reversed: bool,
    ) -> SimplePolygonGeometry {
        SimplePolygonGeometry {
            id: w.id,
            info: w.info,
            tags: tgs,
            refs: w.refs,
            lonlats: lonlats,
            quadtree: w.quadtree,
            area: area,
            layer: layer,
            z_order: z_order,
            minzoom: None,
            reversed: reversed,
        }
    }
/*
    pub fn to_geo(&self, transform: bool) -> geo::Polygon<f64> {
        geo::Polygon::new(
            self.lonlats.iter().map(|l| l.to_xy(transform)).collect(),
            Vec::new(),
        )
    }*/
    pub fn to_wkb(&self, transform: bool, with_srid: bool) -> std::io::Result<Vec<u8>> {
        let mut res = prep_wkb(transform, with_srid, 3, 4 + 4 + 16 * self.lonlats.len())?;
        write_uint32(&mut res, 1)?;
        write_ring(
            &mut res,
            self.lonlats.len(),
            self.lonlats.iter().map(|l| l.to_xy(transform)),
        )?;
        Ok(res)
    }

    

    pub fn to_geometry_geojson(&self, transform: bool) -> std::io::Result<Value> {
        let mut res = Map::new();

        res.insert(String::from("type"), json!("Polygon"));
        res.insert(
            String::from("coordinates"),
            json!(vec![read_lonlats(&self.lonlats, self.reversed, transform)]),
        );
        Ok(json!(res))
    }
}

impl GeoJsonable for SimplePolygonGeometry {
    fn to_geojson(&self, transform: bool) -> std::io::Result<Value> {
        let mut res = Map::new();
        res.insert(String::from("type"), json!("Feature"));
        res.insert(String::from("id"), json!(self.id));
        res.insert(
            String::from("quadtree"),
            json!(self.quadtree.as_tuple().xyz()),
        );
        res.insert(String::from("properties"), pack_tags(&self.tags)?);
        res.insert(String::from("geometry"), self.to_geometry_geojson(transform)?);
        res.insert(
            String::from("way_area"),
            json!(f64::round(self.area * 10.0) / 10.0),
        );

        match self.layer {
            None => {}
            Some(l) => {
                res.insert(String::from("layer"), json!(l));
            }
        }
        match self.z_order {
            None => {}
            Some(l) => {
                res.insert(String::from("z_order"), json!(l));
            }
        }
        match self.minzoom {
            None => {}
            Some(l) => {
                res.insert(String::from("minzoom"), json!(l));
            }
        }
        res.insert(String::from("bbox"), pack_bounds(&self.bounds(),transform));

        Ok(json!(res))
    }
}

use osmquadtree::elements::{WithId, WithInfo, WithQuadtree, WithTags,SetCommon};
impl WithId for SimplePolygonGeometry {
    fn get_id(&self) -> i64 {
        self.id
    }
}

impl WithInfo for SimplePolygonGeometry {
    fn get_info<'a>(&'a self) -> &Option<Info> {
        &self.info
    }
}

impl WithTags for SimplePolygonGeometry {
    fn get_tags<'a>(&'a self) -> &'a [Tag] {
        &self.tags
    }
}

impl WithQuadtree for SimplePolygonGeometry {
    fn get_quadtree<'a>(&'a self) -> &'a Quadtree {
        &self.quadtree
    }
}
impl SetCommon for SimplePolygonGeometry {
    fn set_id(&mut self, id: i64) {
        self.id = id;
    }
    fn set_info(&mut self, info: Info) {
        self.info = Some(info);
    }
    fn set_tags(&mut self, tags: Vec<Tag>) {
        self.tags = tags;
    }
    fn set_quadtree(&mut self, quadtree: Quadtree) {
        self.quadtree = quadtree;
    }
}
