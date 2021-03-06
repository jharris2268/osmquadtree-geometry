use osmquadtree::elements::Tag;
use crate::default_style::DEFAULT_GEOMETRY_STYLE;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Result};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PolyTagSpec {
    Exclude(Vec<String>),
    Include(Vec<String>),
    All,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParentTagSpec {
    pub node_keys: Vec<String>,
    pub way_key: String,
    pub way_priority: BTreeMap<String, i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OpType {
    Min,
    Max,
    List,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelationTagSpec {
    pub source_filter: BTreeMap<String, String>,
    pub source_key: String,
    pub target_key: String,

    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub op_type: OpType,
}
/*
fn get_zorder_value(t: &Tag) -> Option<i64> {
    if t.key == "highway" {
        if t.val == "motorway" {
            return Some(380);
        }
        if t.val == "trunk" {
            return Some(370);
        }
        if t.val == "primary" {
            return Some(360);
        }
        if t.val == "secondary" {
            return Some(350);
        }
        if t.val == "tertiary" {
            return Some(340);
        }
        if t.val == "residential" {
            return Some(330);
        }
        if t.val == "unclassified" {
            return Some(330);
        }
        if t.val == "road" {
            return Some(330);
        }
        if t.val == "living_street" {
            return Some(320);
        }
        if t.val == "pedestrian" {
            return Some(310);
        }
        if t.val == "raceway" {
            return Some(300);
        }
        if t.val == "motorway_link" {
            return Some(240);
        }
        if t.val == "trunk_link" {
            return Some(230);
        }
        if t.val == "primary_link" {
            return Some(220);
        }
        if t.val == "secondary_link" {
            return Some(210);
        }
        if t.val == "tertiary_link" {
            return Some(200);
        }
        if t.val == "service" {
            return Some(150);
        }
        if t.val == "track" {
            return Some(110);
        }
        if t.val == "path" {
            return Some(100);
        }
        if t.val == "footway" {
            return Some(100);
        }
        if t.val == "bridleway" {
            return Some(100);
        }
        if t.val == "cycleway" {
            return Some(100);
        }
        if t.val == "steps" {
            return Some(90);
        }
        if t.val == "platform" {
            return Some(90);
        }
        if t.val == "construction" {
            return Some(10);
        }
        return None;
    }

    if t.key == "railway" {
        if t.val == "rail" {
            return Some(440);
        }
        if t.val == "subway" {
            return Some(420);
        }
        if t.val == "narrow_gauge" {
            return Some(420);
        }
        if t.val == "light_rail" {
            return Some(420);
        }
        if t.val == "funicular" {
            return Some(420);
        }
        if t.val == "preserved" {
            return Some(420);
        }
        if t.val == "monorail" {
            return Some(420);
        }
        if t.val == "miniature" {
            return Some(420);
        }
        if t.val == "turntable" {
            return Some(420);
        }
        if t.val == "tram" {
            return Some(410);
        }
        if t.val == "disused" {
            return Some(400);
        }
        if t.val == "construction" {
            return Some(400);
        }
        if t.val == "platform" {
            return Some(90);
        }
        return None;
    }

    if t.key == "aeroway" {
        if t.val == "runway" {
            return Some(60);
        }
        if t.val == "taxiway" {
            return Some(50);
        }
        return None;
    }
    return None;
}*/

#[derive(Serialize, Deserialize, Debug)]
pub struct GeometryStyle {
    pub feature_keys: BTreeSet<String>,
    pub other_keys: Option<BTreeSet<String>>,
    pub polygon_tags: BTreeMap<String, PolyTagSpec>,
    pub parent_tags: BTreeMap<String, ParentTagSpec>,
    pub relation_tag_spec: Vec<RelationTagSpec>,
    pub z_order_spec: BTreeMap<String,BTreeMap<String,i64>>,
    pub all_objs: bool,
    pub drop_keys: BTreeSet<String>,
    pub multipolygons: bool,
    pub boundary_relations: bool,
}

impl GeometryStyle {
    
    pub fn from_json(input_str: &str) -> Result<GeometryStyle> {
        match serde_json::from_str(input_str) {
            Ok(g) => Ok(g),
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string()))
        }
    }
    
    pub fn default() -> GeometryStyle {
        serde_json::from_str(&DEFAULT_GEOMETRY_STYLE).expect("!!")
    }

    pub fn from_file(infn: &str) -> Result<GeometryStyle> {
        let ff = File::open(infn)?;
        let mut fbuf = BufReader::new(ff);
        match serde_json::from_reader(&mut fbuf) {
            Ok(p) => Ok(p),
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        }
    }
    fn has_feature_key(&self, tags: &[Tag]) -> bool {
        for t in tags {
            if self.feature_keys.contains(&t.key) {
                return true;
            }
        }
        false
    }
    fn has_key(&self, k: &str) -> bool {
        match &self.other_keys {
            None => {
                return true;
            }
            Some(o) => {
                if o.contains(k) {
                    return true;
                }
            }
        }

        self.feature_keys.contains(k)
    }

    fn is_drop(&self, k: &str) -> bool {
        if self.drop_keys.is_empty() {
            return false;
        }

        if self.drop_keys.contains(k) {
            return true;
        }

        match k.find(':') {
            None => false,
            Some(x) => self.drop_keys.contains(&k[0..x + 1]),
        }
    }
    fn get_zorder_value(&self, t: &Tag) -> Option<i64> {
        if let Some(p) = self.z_order_spec.get(&t.key) {
            if let Some(q) = p.get(&t.val) {
                return Some(*q);
            } else if let Some(q) = p.get("*") {
                return Some(*q);
            }
        }
        None
    }
    
    fn filter_tags(&self, tags: &[Tag]) -> (Vec<Tag>, Option<i64>, Option<i64>) {
        let mut res = Vec::new();
        let mut z_order: Option<i64> = None;
        let mut layer: Option<i64> = None;
        for t in tags {
            if self.has_key(&t.key) {
                if !self.is_drop(&t.key) {
                    res.push(t.clone());
                }
            }

            if t.key == "layer" {
                match t.val.parse::<i64>() {
                    Ok(l) => {
                        layer = Some(l);
                    }
                    Err(_) => {}
                }
            }
            match self.get_zorder_value(&t) {
                None => {}
                Some(nv) => {
                    z_order = match z_order {
                        Some(cv) => Some(i64::max(nv, cv)),
                        None => Some(nv),
                    };
                }
            }
            //z_order = i64::max(z_order, get_zorder_value(&t));
        }
        (res, z_order, layer)
    }

    fn check_polygon_tags(&self, tags: &[Tag]) -> bool {
        for t in tags {
            match self.polygon_tags.get(&t.key) {
                None => {}
                Some(pt) => match pt {
                    PolyTagSpec::All => {
                        return true;
                    }
                    PolyTagSpec::Exclude(exc) => {
                        if !exc.contains(&t.val) {
                            return true;
                        }
                    }
                    PolyTagSpec::Include(inc) => {
                        if inc.contains(&t.val) {
                            return true;
                        }
                    }
                },
            }
        }
        return false;
    }

    pub fn process_multipolygon_relation(
        &self,
        tags: &[Tag],
    ) -> Result<(Vec<Tag>, Option<i64>, Option<i64>)> {
        if !self.all_objs && !self.has_feature_key(&tags) {
            return Err(Error::new(ErrorKind::Other, "not a feature"));
        }

        /*if !self.check_polygon_tags(&tags) {
            return Err(Error::new(ErrorKind::Other, "not a polygon feature"));
        }*/

        Ok(self.filter_tags(tags))
    }

    pub fn process_way(
        &self,
        tags: &[Tag],
        is_ring: bool,
    ) -> Result<(bool, Vec<Tag>, Option<i64>, Option<i64>)> {
        if !self.all_objs && !self.has_feature_key(&tags) {
            return Err(Error::new(ErrorKind::Other, "not a feature"));
        }
        let is_poly = is_ring && self.check_polygon_tags(&tags);

        let (t, z, l) = self.filter_tags(tags);
        Ok((is_poly, t, z, l))
    }

    pub fn process_node(&self, tags: &[Tag]) -> Result<(Vec<Tag>, Option<i64>)> {
        if !self.all_objs && !self.has_feature_key(&tags) {
            return Err(Error::new(ErrorKind::Other, "not a feature"));
        }

        let (t, _, lyr) = self.filter_tags(tags);
        Ok((t, lyr))
    }
}
