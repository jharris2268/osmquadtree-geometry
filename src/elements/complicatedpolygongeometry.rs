use osmquadtree::elements::{Bbox, Info, Quadtree, Relation, Tag};

use crate::elements::pointgeometry::pack_tags;
use crate::elements::simplepolygongeometry::{pack_bounds, read_lonlats};
use crate::elements::{GeoJsonable,WithBounds};
use crate::position::calc_ring_area_and_bbox;
use crate::wkb::{prep_wkb, write_ring, write_uint32, /*AsWkb*/};
use crate::LonLat;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::fmt;
use std::io::{Error, ErrorKind, Result};
//extern crate geo;

#[derive(Serialize, Clone)]
pub struct RingPart {
    pub orig_id: i64,
    pub is_reversed: bool,
    pub refs: Vec<i64>,
    pub lonlats: Vec<LonLat>,
}
impl fmt::Debug for RingPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("orig_id", &self.orig_id)
            .field("is_reversed", &self.is_reversed)
            .field("np", &self.refs.len())
            .field("first", &self.refs[0])
            .field("last", &self.refs[self.refs.len() - 1])
            .finish()
    }
}

impl RingPart {
    pub fn empty() -> RingPart {
        RingPart::new(0,false,Vec::new(),Vec::new())
    }
    pub fn new(orig_id: i64, is_reversed: bool, refs: Vec<i64>, lonlats: Vec<LonLat>) -> RingPart {
        RingPart {
            orig_id,
            is_reversed,
            refs,
            lonlats,
        }
    }
}

#[derive(Debug, Serialize,Clone)]
pub struct Ring {
    
    pub parts: Vec<RingPart>,
    pub area: f64,
    pub bbox: Bbox,
    //pub geo: Option<geo::LineString<f64>>,
}

impl Ring {
    
    
    pub fn new() -> Ring {
        Ring {
            parts: Vec::new(),
            area: 0.0,
            bbox: Bbox::empty(),
            //geo: None,
        }
    }

    pub fn calc_area_bbox(&mut self) -> Result<()> {
        let x = calc_ring_area_and_bbox(&self.lonlats()?);
        self.area = x.0;
        self.bbox = x.1;
        //self.geo = Some(self.to_geo(false));
        Ok(())
    }

    pub fn reverse(&mut self) {
        self.parts.reverse();
        for p in self.parts.iter_mut() {
            p.is_reversed = !p.is_reversed;
        }
        self.area *= -1.0;
    }

    pub fn len(&self) -> usize {
        if self.parts.is_empty() {
            return 0;
        }
        let mut r = 0;
        for p in &self.parts {
            if r > 0 {
                r -= 1;
            }
            r += p.lonlats.len();
        }
        r
    }

    pub fn first_last(&self) -> (i64, i64) {
        let p = &self.parts[0];
        let f = if p.is_reversed {
            p.refs[p.refs.len() - 1]
        } else {
            p.refs[0]
        };

        let q = &self.parts[self.parts.len() - 1];
        let t = if q.is_reversed {
            q.refs[0]
        } else {
            q.refs[q.refs.len() - 1]
        };
        (f, t)
    }

    pub fn is_ring(&self) -> bool {
        let (f, t) = self.first_last();
        f == t
    }

    /*pub fn refs<'a>(&'a self) -> Result<Vec<&'a i64>> {
        let mut res = Vec::new();
        for p in &self.parts {
            if p.is_reversed {
                let mut ii = p.refs.iter().rev();

                if !res.is_empty() {
                    let f = ii.next().unwrap();
                    if res[res.len() - 1] != f {
                        return Err(Error::new(ErrorKind::Other, "not a ring"));
                    }
                }
                res.extend(ii);
            } else {
                let mut ii = p.refs.iter();

                if !res.is_empty() {
                    let f = ii.next().unwrap();
                    if res[res.len() - 1] != f {
                        return Err(Error::new(ErrorKind::Other, "not a ring"));
                    }
                }
                res.extend(ii);
            }
        }
        if res[0] != res[res.len() - 1] {
            return Err(Error::new(ErrorKind::Other, "not a ring"));
        }

        Ok(res)
    }
    */
    pub fn refs(&self) -> Result<Vec<i64>> {
        let mut res = Vec::new();
        for p in &self.parts {
            if p.is_reversed {
                let mut ii = p.refs.iter().rev();

                if !res.is_empty() {
                    let f = ii.next().unwrap();
                    if res[res.len() - 1] != *f {
                        return Err(Error::new(ErrorKind::Other, "not a ring"));
                    }
                }
                res.extend(ii);
            } else {
                let mut ii = p.refs.iter();

                if !res.is_empty() {
                    let f = ii.next().unwrap();
                    if res[res.len() - 1] != *f {
                        return Err(Error::new(ErrorKind::Other, "not a ring"));
                    }
                }
                res.extend(ii);
            }
        }
        if res[0] != res[res.len() - 1] {
            return Err(Error::new(ErrorKind::Other, "not a ring"));
        }

        Ok(res)
    }
    pub fn lonlats(&self) -> Result<Vec<LonLat>> {
        let mut res = Vec::new();
        for p in &self.parts {
            if p.is_reversed {
                let mut ii = p.lonlats.iter().rev();

                if !res.is_empty() {
                    let f = ii.next().unwrap();
                    if &res[res.len() - 1] != f {
                        return Err(Error::new(ErrorKind::Other, "not a ring"));
                    }
                }
                res.extend(ii.map(|l| l.clone()));
            } else {
                let mut ii = p.lonlats.iter();

                if !res.is_empty() {
                    let f = ii.next().unwrap();
                    if &res[res.len() - 1] != f {
                        return Err(Error::new(ErrorKind::Other, "not a ring"));
                    }
                }
                res.extend(ii.map(|l| l.clone()));
            }
        }
        if res[0] != res[res.len() - 1] {
            return Err(Error::new(ErrorKind::Other, "not a ring"));
        }

        Ok(res)
    }

    pub fn lonlats_iter<'a>(&'a self) -> RingLonLatsIter<'a> {
        RingLonLatsIter::new(self)
    }

    /*pub fn to_geo(&self, transform: bool) -> geo::LineString<f64> {
        if !transform && !self.geo.is_none() {
            return self.geo.as_ref().unwrap().clone();
        }
        geo::LineString(self.lonlats_iter().map(|l| l.to_xy(transform)).collect())
    }*/
}

pub struct RingLonLatsIter<'a> {
    ring: &'a Ring,
    part_idx: usize,
    coord_idx: usize,
}

impl<'a> RingLonLatsIter<'a> {
    pub fn new(ring: &'a Ring) -> RingLonLatsIter<'a> {
        RingLonLatsIter {
            ring: ring,
            part_idx: 0,
            coord_idx: 0,
        }
    }

    fn curr(&self) -> Option<&'a LonLat> {
        if self.part_idx >= self.ring.parts.len() {
            return None;
        }

        let p = &self.ring.parts[self.part_idx];

        if p.is_reversed {
            Some(&p.lonlats[p.lonlats.len() - 1 - self.coord_idx])
        } else {
            Some(&p.lonlats[self.coord_idx])
        }
    }

    fn next(&mut self) {
        if self.part_idx >= self.ring.parts.len() {
            return;
        }
        self.coord_idx += 1;
        while self.coord_idx == self.ring.parts[self.part_idx].lonlats.len() {
            self.part_idx += 1;
            if self.part_idx >= self.ring.parts.len() {
                return;
            }
            self.coord_idx = 1;
        }
    }
}

impl<'a> Iterator for RingLonLatsIter<'a> {
    type Item = &'a LonLat;

    fn next(&mut self) -> Option<&'a LonLat> {
        match self.curr() {
            None => None,
            Some(r) => {
                self.next();
                Some(r)
            }
        }
    }
}

fn merge_rings(rings: &mut Vec<Ring>) -> (bool, Option<Ring>) {
    if rings.len() == 0 {
        return (false, None);
    }
    if rings.len() == 1 {
        if rings[0].is_ring() {
            let zz = rings.remove(0);
            return (true, Some(zz));
        }
        return (false, None);
    }

    for i in 0..rings.len() - 1 {
        let (f, t) = rings[i].first_last();
        if f == t {
            let zz = rings.remove(i);
            return (true, Some(zz));
        }
        for j in i + 1..rings.len() {
            let (g, u) = rings[j].first_last();

            if t == g {
                let zz = rings.remove(j);
                rings[i].parts.extend(zz.parts);
                if rings[i].is_ring() {
                    let zz = rings.remove(i);
                    return (true, Some(zz));
                }
                return (true, None);
            } else if t == u {
                let mut zz = rings.remove(j);
                zz.reverse();
                rings[i].parts.extend(zz.parts);
                if rings[i].is_ring() {
                    let zz = rings.remove(i);
                    return (true, Some(zz));
                }
                return (true, None);
            } else if f == u {
                let mut zz = rings.remove(j);
                zz.reverse();
                rings[i].reverse();
                rings[i].parts.extend(zz.parts);
                return (true, None);
            } else if f == g {
                let zz = rings.remove(j);
                rings[i].reverse();
                rings[i].parts.extend(zz.parts);

                return (true, None);
            }
        }
    }
    return (false, None);
}

pub fn collect_rings(ww: Vec<RingPart>) -> Result<(Vec<Ring>, Vec<RingPart>)> {
    //let nw=ww.len();
    let mut parts = Vec::new();
    for w in ww {
        let mut r = Ring::new();
        r.parts.push(w);
        parts.push(r);
    }

    let mut res = Vec::new();
    loop {
        let (f, r) = merge_rings(&mut parts);
        match r {
            None => {}
            Some(r) => {
                res.push(r);
            }
        }
        if !f {
            break;
        }
    }

    let mut rem = Vec::new();
    for p in parts {
        for q in p.parts {
            rem.push(q);
        }
    }

    Ok((res, rem))
}

#[derive(Debug, Serialize,Clone)]
pub struct PolygonPart {
    pub exterior: Ring,
    pub interiors: Vec<Ring>,

    pub area: f64,
}

impl PolygonPart {
    
    pub fn empty() -> PolygonPart {
        PolygonPart{ exterior: Ring::new(), interiors: Vec::new(), area: 0.0 }
    }
    
    pub fn new(mut ext: Ring) -> PolygonPart {
        if ext.area < 0.0 {
            ext.reverse();
        }
        let a = ext.area;
        PolygonPart {
            exterior: ext,
            interiors: Vec::new(),
            area: a,
        }
    }

    pub fn add_interior(&mut self, mut p: Ring) {
        if p.area > 0.0 {
            p.reverse();
        }
        self.area += p.area;
        self.interiors.push(p);
    }

    pub fn prep_coordinates(&self, transform: bool) -> Result<Vec<Vec<(f64, f64)>>> {
        let mut rings = Vec::new();

        rings.push(read_lonlats(&self.exterior.lonlats()?, false, transform));
        for ii in &self.interiors {
            rings.push(read_lonlats(&ii.lonlats()?, false, transform));
        }

        Ok(rings)
    }
    pub fn to_wkb(&self, transform: bool, with_srid: bool) -> Result<Vec<u8>> {
        let mut res = prep_wkb(transform, with_srid, 3, 0)?;

        write_uint32(&mut res, 1 + self.interiors.len() as u32)?;
        write_ring(
            &mut res,
            self.exterior.len(),
            self.exterior.lonlats_iter().map(|l| l.to_xy(transform)),
        )?;
        for ii in &self.interiors {
            write_ring(
                &mut res,
                ii.len(),
                ii.lonlats_iter().map(|l| l.to_xy(transform)),
            )?;
        }
        Ok(res)
    }
}

#[derive(Debug, Serialize,Clone)]
pub struct ComplicatedPolygonGeometry {
    pub id: i64,
    pub info: Option<Info>,
    pub tags: Vec<Tag>,
    pub parts: Vec<PolygonPart>,
    pub z_order: Option<i64>,
    pub layer: Option<i64>,
    pub area: f64,
    pub minzoom: Option<i64>,
    pub quadtree: Quadtree,
}
impl WithBounds for ComplicatedPolygonGeometry {
    fn bounds(&self) -> Bbox {
        let mut res = Bbox::empty();
        for p in &self.parts {
            for l in &p.exterior.lonlats().unwrap() {
                res.expand(l.lon, l.lat);
            }
        }
        res
    }
}


impl ComplicatedPolygonGeometry {
    pub fn empty() -> ComplicatedPolygonGeometry {
        ComplicatedPolygonGeometry{id: 0, info: None, tags: Vec::new(), parts: Vec::new(),
            area: 0.0, layer: None, z_order: None, minzoom: None, quadtree: Quadtree::empty()}
    }
    
    
    pub fn new(
        relation: &Relation,
        tags: Vec<Tag>,
        z_order: Option<i64>,
        layer: Option<i64>,
        parts: Vec<PolygonPart>,
    ) -> ComplicatedPolygonGeometry {
        let mut area = 0.0;
        for p in &parts {
            area += p.area;
        }

        ComplicatedPolygonGeometry {
            id: relation.id,
            info: relation.info.clone(),
            tags: tags,
            parts: parts,
            z_order: z_order,
            layer: layer,
            area: area,
            minzoom: None,
            quadtree: relation.quadtree,
        }
    }
/*
    pub fn to_geo(&self, transform: bool) -> geo::MultiPolygon<f64> {
        let mut polys = Vec::new();
        for p in &self.parts {
            //let ext = p.exterior.lonlats().unwrap().iter().map(|l| { l.to_xy(transform) }).collect();
            //let ext = p.exterior.lonlats_iter().map(|l| { l.to_xy(transform) }).collect();
            let ext = p.exterior.to_geo(transform);
            let mut ints = Vec::new();
            for ii in &p.interiors {
                //ints.push(ii.lonlats().unwrap().iter().map(|l| { l.to_xy(transform) }).collect());
                //ints.push(ii.lonlats_iter().map(|l| { l.to_xy(transform) }).collect());
                ints.push(ii.to_geo(transform));
            }
            polys.push(geo::Polygon::new(ext, ints));
        }
        geo::MultiPolygon(polys)
    }
*/
    pub fn to_wkb(&self, transform: bool, with_srid: bool) -> std::io::Result<Vec<u8>> {
        /*let xx = self.to_geo(transform);
        let srid = if with_srid {
            Some(if transform { 3857 } else { 4326 })
        } else {
            None
        };
        xx.as_wkb(srid)
        */
        

        if self.parts.len()==1 {
            self.parts[0].to_wkb(transform, with_srid)

        } else {
            let mut res = prep_wkb(transform, with_srid, 6, 4)?;
            write_uint32(&mut res, self.parts.len() as u32)?;
            for p in &self.parts {
                res.extend(p.to_wkb(transform, with_srid)?);
            }

            Ok(res)
        }
    }

    

    pub fn to_geometry_geojson(&self, transform: bool) -> std::io::Result<Value> {
        let mut res = Map::new();
        if self.parts.len() == 1 {
            res.insert(String::from("type"), json!("Polygon"));
            res.insert(
                String::from("coordinates"),
                json!(self.parts[0].prep_coordinates(transform)?),
            );
        } else {
            res.insert(String::from("type"), json!("MultiPolygon"));
            let mut cc = Vec::new();
            for p in &self.parts {
                cc.push(p.prep_coordinates(transform)?);
            }
            res.insert(String::from("coordinates"), json!(cc));
        }
        Ok(json!(res))
    }
}

impl GeoJsonable for ComplicatedPolygonGeometry {
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
impl WithId for ComplicatedPolygonGeometry {
    fn get_id(&self) -> i64 {
        self.id
    }
}

impl WithTags for ComplicatedPolygonGeometry {
    fn get_tags<'a>(&'a self) -> &'a [Tag] {
        &self.tags
    }
}

impl WithInfo for ComplicatedPolygonGeometry {
    fn get_info<'a>(&'a self) -> &Option<Info> {
        &self.info
    }
}

impl WithQuadtree for ComplicatedPolygonGeometry {
    fn get_quadtree<'a>(&'a self) -> &'a Quadtree {
        &self.quadtree
    }
}
impl SetCommon for ComplicatedPolygonGeometry {
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
