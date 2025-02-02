use crate::{
    ComplicatedPolygonGeometry, GeometryBlock, LinestringGeometry, PointGeometry, PolygonPart,
    Ring, RingPart, SimplePolygonGeometry, LonLat, WithBounds, CallFinishGeometryBlock,
    Timings, Error, Result
};

use osmquadtree::elements::{pack_head, PackStringTable, read_stringtable, read_common, Quadtree, Bbox};
use osmquadtree::mergechanges::{read_filter,Poly};

use simple_protocolbuffers::{
    data_length, pack_data, pack_delta_int, pack_delta_int_ref, pack_value, zig_zag, un_zig_zag, PbfTag, IterTags, read_delta_packed_int
};


fn pack_all(tag: u64, objs: Vec<Vec<u8>>) -> Vec<u8> {
    if objs.is_empty() {
        return Vec::new();
    }

    let mut t = 0;
    for vi in &objs {
        t += data_length(20, vi.len());
    }

    let mut res = Vec::with_capacity(t);
    for vi in objs {
        pack_data(&mut res, tag, &vi);
    }
    res
}

fn pack_point_geometry(pst: &mut Box<PackStringTable>, geom: &PointGeometry) -> Result<Vec<u8>> {
    let mut res = Vec::with_capacity(95 + 10 * geom.tags.len());

    pack_head(&geom.id, &geom.info, &geom.tags, &mut res, pst)?;
    pack_value(&mut res, 8, zig_zag(geom.lonlat.lat as i64));
    pack_value(&mut res, 9, zig_zag(geom.lonlat.lon as i64));

    pack_value(&mut res, 20, zig_zag(geom.quadtree.as_int()));

    match geom.minzoom {
        None => {}
        Some(m) => pack_value(&mut res, 22, m as u64),
    }
    match geom.layer {
        None => {}
        Some(l) => pack_value(&mut res, 24, zig_zag(l)),
    }
    Ok(res)
}


fn unpack_point_geometry(strings: &Vec<String>, data: &[u8]) -> Result<PointGeometry> {
    
    let mut geom = PointGeometry::empty();
    
    let rem = read_common(&mut geom, &strings, &data, false)?;
    
    for tg in rem {
        match tg {
            PbfTag::Value(8, lat) => { geom.lonlat.lat = un_zig_zag(lat) as i32; },
            PbfTag::Value(9, lon) => { geom.lonlat.lon = un_zig_zag(lon) as i32; },
            PbfTag::Value(22, m) => { geom.minzoom = Some(m as i64); },
            PbfTag::Value(24, l) => { geom.layer = Some(un_zig_zag(l)); },
            _ => {},
        }
    }
    Ok(geom)
} 

fn pack_linestring_geometry(
    pst: &mut Box<PackStringTable>,
    geom: &LinestringGeometry,
) -> Result<Vec<u8>> {
    let mut res = Vec::with_capacity(95 + 10 * geom.tags.len() + 25 * geom.refs.len());

    pack_head(&geom.id, &geom.info, &geom.tags, &mut res, pst)?;
    pack_data(&mut res, 8, &pack_delta_int_ref(geom.refs.iter()));
    match geom.z_order {
        None => {}
        Some(z) => {
            pack_value(&mut res, 12, zig_zag(z));
        }
    }
    pack_data(
        &mut res,
        13,
        &pack_delta_int(geom.lonlats.iter().map(|l| l.lon as i64)),
    );
    pack_data(
        &mut res,
        14,
        &pack_delta_int(geom.lonlats.iter().map(|l| l.lat as i64)),
    );
    pack_value(&mut res, 15, zig_zag((geom.length * 100.0).round() as i64));

    pack_value(&mut res, 20, zig_zag(geom.quadtree.as_int()));

    match geom.minzoom {
        None => {}
        Some(m) => pack_value(&mut res, 22, m as u64),
    }
    match geom.layer {
        None => {}
        Some(l) => pack_value(&mut res, 24, zig_zag(l)),
    }
    Ok(res)
}
  
fn unpack_linestring_geometry(strings: &Vec<String>, data: &[u8]) -> Result<LinestringGeometry> {
    
    let mut geom = LinestringGeometry::empty();
    
    let rem = read_common(&mut geom, &strings, &data, false)?;
    let mut lats = Vec::new();
    let mut lons = Vec::new();
    for tg in rem {
        match tg {
            PbfTag::Data(8, d) => { geom.refs = read_delta_packed_int(&d); },
            PbfTag::Value(12, zo) => { geom.z_order = Some(un_zig_zag(zo)); },
            PbfTag::Data(13, d) => { lons = read_delta_packed_int(&d);},
            PbfTag::Data(14, d) => { lats = read_delta_packed_int(&d); },
            PbfTag::Value(15, l) => { geom.length = un_zig_zag(l) as f64 / 100.0; },
            PbfTag::Value(22, m) => { geom.minzoom = Some(m as i64); },
            PbfTag::Value(24, l) => { geom.layer = Some(un_zig_zag(l)); },
            _ => {},
        }
    }
    geom.lonlats = set_lon_lats(lons, lats)?;
    
        
    Ok(geom)
} 
fn set_lon_lats(lons: Vec<i64>, lats: Vec<i64>) -> Result<Vec<LonLat>> {
    if lons.len()!=lats.len() {
        return Err(Error::InvalidDataError(format!("lons.len()!=lats.len()")));
    }
    
    let mut res = Vec::with_capacity(lons.len());
    for i in 0..lons.len() {
        res.push(LonLat::new(lons[i] as i32, lats[i] as i32));
    }
    Ok(res)
}

fn pack_simple_polygon_geometry(
    pst: &mut Box<PackStringTable>,
    geom: &SimplePolygonGeometry,
) -> Result<Vec<u8>> {
    let mut res = Vec::with_capacity(95 + 10 * geom.tags.len() + 25 * geom.refs.len());

    pack_head(&geom.id, &geom.info, &geom.tags, &mut res, pst)?;
    pack_data(&mut res, 8, &pack_delta_int_ref(geom.refs.iter()));
    match geom.z_order {
        None => {}
        Some(z) => {
            pack_value(&mut res, 12, zig_zag(z));
        }
    }
    pack_data(
        &mut res,
        13,
        &pack_delta_int(geom.lonlats.iter().map(|l| l.lon as i64)),
    );
    pack_data(
        &mut res,
        14,
        &pack_delta_int(geom.lonlats.iter().map(|l| l.lat as i64)),
    );
    pack_value(&mut res, 16, zig_zag((geom.area * 100.0).round() as i64));

    pack_value(&mut res, 20, zig_zag(geom.quadtree.as_int()));

    match geom.minzoom {
        None => {}
        Some(m) => {
            pack_value(&mut res, 22, m as u64);
        }
    }
    if geom.reversed {
        pack_value(&mut res, 23, 1);
    }
    match geom.layer {
        None => {}
        Some(l) => {
            pack_value(&mut res, 24, zig_zag(l));
        }
    }
    Ok(res)
}
fn unpack_simplepolygon_geometry(strings: &Vec<String>, data: &[u8]) -> Result<SimplePolygonGeometry> {
    
    let mut geom = SimplePolygonGeometry::empty();
    
    let rem = read_common(&mut geom, &strings, &data, false)?;
    let mut lats = Vec::new();
    let mut lons = Vec::new();
    for tg in rem {
        match tg {
            PbfTag::Data(8, d) => { geom.refs = read_delta_packed_int(&d); },
            PbfTag::Value(12, zo) => { geom.z_order = Some(un_zig_zag(zo)); },
            PbfTag::Data(13, d) => { lons = read_delta_packed_int(&d);},
            PbfTag::Data(14, d) => { lats = read_delta_packed_int(&d); },
            PbfTag::Value(16, a) => { geom.area = un_zig_zag(a) as f64 / 100.0; },
            PbfTag::Value(22, m) => { geom.minzoom = Some(m as i64); },
            PbfTag::Value(23, r) => { geom.reversed = r==1; },
            PbfTag::Value(24, l) => { geom.layer = Some(un_zig_zag(l)); },
            _ => {},
        }
    }
    
    geom.lonlats = set_lon_lats(lons, lats)?;
    
        
    Ok(geom)
} 
fn pack_ringpart(rp: &RingPart) -> Result<Vec<u8>> {
    let mut res = Vec::with_capacity(40 + 25 * rp.refs.len());

    pack_value(&mut res, 1, rp.orig_id as u64);
    pack_data(&mut res, 2, &pack_delta_int_ref(rp.refs.iter()));
    pack_data(
        &mut res,
        3,
        &pack_delta_int(rp.lonlats.iter().map(|l| l.lon as i64)),
    );
    pack_data(
        &mut res,
        4,
        &pack_delta_int(rp.lonlats.iter().map(|l| l.lat as i64)),
    );
    if rp.is_reversed {
        pack_value(&mut res, 5, 1);
    }
    Ok(res)
}

fn unpack_ringpart(data: &[u8]) -> Result<RingPart> {
    let mut rp = RingPart::empty();
    let mut lats = Vec::new();
    let mut lons = Vec::new();
    for tg in IterTags::new(&data) {
        match tg {
            PbfTag::Value(1, i) => { rp.orig_id = i as i64; },
            PbfTag::Data(2, d) => { rp.refs = read_delta_packed_int(&d); },
            PbfTag::Data(3, d) => { lons = read_delta_packed_int(&d);},
            PbfTag::Data(4, d) => { lats = read_delta_packed_int(&d); },
            PbfTag::Value(5, r) => { rp.is_reversed = r==1; },
            _ => {},
        }
    }
    rp.lonlats = set_lon_lats(lons, lats)?;
        
    Ok(rp)
} 
    

fn pack_ring(rr: &Ring) -> Result<Vec<u8>> {
    let mut tl = 0;
    let mut parts = Vec::with_capacity(rr.parts.len());
    for p in &rr.parts {
        let q = pack_ringpart(p)?;
        tl += data_length(1, q.len());
        parts.push(q);
    }

    let mut res = Vec::with_capacity(tl);
    for p in parts {
        pack_data(&mut res, 1, &p);
    }
    Ok(res)
}

fn unpack_ring(data: &[u8]) -> Result<Ring> {
    let mut res = Ring::new();
    for tg in IterTags::new(&data) {
        match tg {
            PbfTag::Data(1, d) => { res.parts.push(unpack_ringpart(&d)?); },
            _ => {}
        }
    }
    res.calc_area_bbox()?;
    Ok(res)
}


fn pack_polygon_part(idx: usize, part: &PolygonPart) -> Result<Vec<u8>> {
    let mut tl = 20;

    let extr = pack_ring(&part.exterior)?;
    tl += data_length(2, extr.len());

    let mut intrs = Vec::with_capacity(part.interiors.len());
    for ii in &part.interiors {
        let intr = pack_ring(&ii)?;
        tl += data_length(2, intr.len());
        intrs.push(intr);
    }

    let mut res = Vec::with_capacity(tl);
    pack_value(&mut res, 1, idx as u64);
    pack_data(&mut res, 2, &extr);
    for ii in intrs {
        pack_data(&mut res, 3, &ii);
    }

    pack_value(&mut res, 4, zig_zag((part.area * 100.0).round() as i64));

    Ok(res)
}

fn unpack_polygon_part(data: &[u8]) -> Result<(usize, PolygonPart)> {
    let mut idx=0;
    let mut part = PolygonPart::empty();
    
    for tg in IterTags::new(&data) {
        match tg {
            PbfTag::Value(1, i) => { idx = i as usize; },
            PbfTag::Data(2, d) => { part.exterior = unpack_ring(&d)?; },
            PbfTag::Data(3, d) => { part.interiors.push(unpack_ring(&d)?); },
            PbfTag::Value(4, a) => { part.area = un_zig_zag(a) as f64 / 100.0; },
            _ => {},
        }
    }
    Ok((idx, part))
}

fn pack_complicated_polygon_geometry(
    pst: &mut Box<PackStringTable>,
    geom: &ComplicatedPolygonGeometry,
) -> Result<Vec<u8>> {
    let mut packed_parts = Vec::with_capacity(geom.parts.len());
    let mut pl = 0;
    for (i, p) in geom.parts.iter().enumerate() {
        let q = pack_polygon_part(i, p)?;
        pl += data_length(25, q.len());
        packed_parts.push(q);
    }

    let mut res = Vec::with_capacity(95 + 10 * geom.tags.len() + pl);

    pack_head(&geom.id, &geom.info, &geom.tags, &mut res, pst)?;
    match geom.z_order {
        None => {}
        Some(z) => {
            pack_value(&mut res, 12, zig_zag(z));
        }
    }
    pack_value(&mut res, 16, zig_zag((geom.area * 100.0).round() as i64));

    pack_value(&mut res, 20, zig_zag(geom.quadtree.as_int()));

    match geom.minzoom {
        None => {}
        Some(m) => pack_value(&mut res, 22, m as u64),
    }

    match geom.layer {
        None => {}
        Some(l) => pack_value(&mut res, 24, zig_zag(l)),
    }

    for p in packed_parts {
        pack_data(&mut res, 25, &p);
    }
    Ok(res)
}

fn unpack_complicated_polygon_geometry(strings: &Vec<String>, data: &[u8]) -> Result<ComplicatedPolygonGeometry> {
    
    let mut geom = ComplicatedPolygonGeometry::empty();
    
    let rem = read_common(&mut geom, &strings, &data, false)?;
    for tg in rem {
        match tg {
            
            PbfTag::Value(12, zo) => { geom.z_order = Some(un_zig_zag(zo)); },
            PbfTag::Value(16, a) => { geom.area = un_zig_zag(a) as f64 / 100.0; },
            PbfTag::Value(22, m) => { geom.minzoom = Some(m as i64); },
            PbfTag::Value(24, l) => { geom.layer = Some(un_zig_zag(l)); },
            PbfTag::Data(25, d) => {
                let (i,p) = unpack_polygon_part(&d)?;
                if i != geom.parts.len() {
                    return Err(Error::InvalidDataError(format!("parts order wrong?")));
                }
                geom.parts.push(p);
            },
                
            _ => {},
        }
    }
        
    Ok(geom)
} 
    


pub fn pack_geometry_block(gb: &GeometryBlock) -> Result<Vec<u8>> {
    let mut pst = Box::new(PackStringTable::new());

    let mut points = Vec::with_capacity(gb.points.len());
    for p in &gb.points {
        points.push(pack_point_geometry(&mut pst, p)?);
    }
    let points_group = pack_all(20, points);

    let mut linestrings = Vec::with_capacity(gb.linestrings.len());
    for p in &gb.linestrings {
        linestrings.push(pack_linestring_geometry(&mut pst, p)?);
    }
    let linestrings_group = pack_all(21, linestrings);

    let mut simple_polygons = Vec::with_capacity(gb.simple_polygons.len());
    for p in &gb.simple_polygons {
        simple_polygons.push(pack_simple_polygon_geometry(&mut pst, p)?);
    }
    let simple_polygons_group = pack_all(22, simple_polygons);

    let mut complicated_polygons = Vec::with_capacity(gb.complicated_polygons.len());
    for p in &gb.complicated_polygons {
        complicated_polygons.push(pack_complicated_polygon_geometry(&mut pst, p)?);
    }
    let complicated_polygons_group = pack_all(23, complicated_polygons);

    let strs = pst.pack();

    let mut res = Vec::with_capacity(
        data_length(1, strs.len())
            + data_length(2, points_group.len())
            + data_length(2, linestrings_group.len())
            + data_length(2, simple_polygons_group.len())
            + data_length(2, complicated_polygons_group.len())
            + 20,
    );

    pack_data(&mut res, 1, &strs);
    if !points_group.is_empty() {
        pack_data(&mut res, 2, &points_group);
    }
    if !linestrings_group.is_empty() {
        pack_data(&mut res, 2, &linestrings_group);
    }
    if !simple_polygons_group.is_empty() {
        pack_data(&mut res, 2, &simple_polygons_group);
    }
    if !complicated_polygons_group.is_empty() {
        pack_data(&mut res, 2, &complicated_polygons_group);
    }
    if !gb.quadtree.is_empty() {
        pack_value(&mut res, 32, zig_zag(gb.quadtree.as_int()));
    }
    if gb.end_date != 0 {
        pack_value(&mut res, 34, gb.end_date as u64)
    }
    Ok(res)
}








fn unpack_group(gb: &mut GeometryBlock, strs: &Vec<String>, data: &[u8]) -> Result<()> {
    
    for tg in IterTags::new(&data) {
        match tg {
            PbfTag::Data(20, d) => { gb.points.push(unpack_point_geometry(&strs, &d)?); },
            PbfTag::Data(21, d) => { gb.linestrings.push(unpack_linestring_geometry(&strs, &d)?); },
            PbfTag::Data(22, d) => { gb.simple_polygons.push(unpack_simplepolygon_geometry(&strs, &d)?); },
            PbfTag::Data(23, d) => { gb.complicated_polygons.push(unpack_complicated_polygon_geometry(&strs, &d)?); },
            _ => {},
        }
    }
    Ok(())
}
    

pub fn unpack_geometry_block(idx: i64, data: &[u8]) -> Result<GeometryBlock> {
    let mut gb = GeometryBlock::new(idx, Quadtree::empty(),  0);
    
    let mut strs = Vec::new();
    for tg in IterTags::new(&data) {
        match tg {
            PbfTag::Data(1, d) => { strs = read_stringtable(&d)?; },
            PbfTag::Data(2, d) => { unpack_group(&mut gb, &strs, &d)?; },
            PbfTag::Value(32, q) => { gb.quadtree = Quadtree::new(un_zig_zag(q)); },
            PbfTag::Value(34, q) => { gb.end_date = q as i64; },
            _ => {}
        }
    }
    Ok(gb)
    
    
}

pub enum GeometryFilter {
    Null,
    Bbox(Bbox),
    Poly(Poly)
}

impl GeometryFilter {
    pub fn check<T: WithBounds>(&self, obj: &T) -> bool {
        match self {
            GeometryFilter::Null => true,
            GeometryFilter::Bbox(b) => b.overlaps(&obj.bounds()),
            GeometryFilter::Poly(p) => p.check_box(&obj.bounds())
        }
    }
    pub fn is_null(&self) -> bool {
        match self {
            GeometryFilter::Null => true,
            GeometryFilter::Bbox(b) => b.is_planet(),
            _ => false
        }
    }
}
            
                    
fn max_minzoom_check(test_minzoom: &Option<i64>, val_minzoom: &Option<i64>) -> bool {
    match (val_minzoom, test_minzoom) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(v), Some(t)) => v <= t
    }
}

fn unpack_group_filter(gb: &mut GeometryBlock, strs: &Vec<String>, data: &[u8], filter: &GeometryFilter, max_minzoom: &Option<i64>) -> Result<()> {
    
    for tg in IterTags::new(&data) {
        match tg {
            PbfTag::Data(20, d) => {
                let p = unpack_point_geometry(&strs, &d)?;
                if filter.check(&p) && max_minzoom_check(max_minzoom, &p.minzoom) { 
                    gb.points.push(p);
                }
            },
            PbfTag::Data(21, d) => {
                let p = unpack_linestring_geometry(&strs, &d)?;
                if filter.check(&p) && max_minzoom_check(max_minzoom, &p.minzoom) { 
                    gb.linestrings.push(p);
                }
            },
            PbfTag::Data(22, d) => {
                let p = unpack_simplepolygon_geometry(&strs, &d)?;
                if filter.check(&p) && max_minzoom_check(max_minzoom, &p.minzoom) { 
                    gb.simple_polygons.push(p);
                }
            },
            PbfTag::Data(23, d) => {
                let p = unpack_complicated_polygon_geometry(&strs, &d)?;
                if filter.check(&p) && max_minzoom_check(max_minzoom, &p.minzoom) { 
                    gb.complicated_polygons.push(p);
                }
            },
            _ => {},
        }
    }
    Ok(())
}
    

pub fn unpack_geometry_block_filter(idx: i64, data: &[u8], filter: &GeometryFilter, max_minzoom: &Option<i64>) -> Result<GeometryBlock> {
    if filter.is_null() && max_minzoom.is_none() {
        return unpack_geometry_block(idx, data);
    }
    
    let mut gb = GeometryBlock::new(idx, Quadtree::empty(),  0);
    
    let mut strs = Vec::new();
    for tg in IterTags::new(&data) {
        match tg {
            PbfTag::Data(1, d) => { strs = read_stringtable(&d)?; },
            PbfTag::Data(2, d) => { unpack_group_filter(&mut gb, &strs, &d, filter, max_minzoom)?; },
            PbfTag::Value(32, q) => { gb.quadtree = Quadtree::new(un_zig_zag(q)); },
            PbfTag::Value(34, q) => { gb.end_date = q as i64; },
            _ => {}
        }
    }
    Ok(gb)
    
    
}

use channelled_callbacks::{/*CallFinish,Timings,CallbackSync,CallbackMerge,*/CallAll};
use osmquadtree::message;
use osmquadtree::pbfformat::{get_file_locs_max_depth,read_all_blocks_parallel_with_progbar,FileBlock};
use std::sync::Arc;


pub fn read_geometry_blocks(
    infn: &str, cb: CallFinishGeometryBlock, filter_str: Option<&str>, max_minzoom: Option<i64>, numchan: usize) -> Result<Timings> {
    
    let (bx, poly) = read_filter(filter_str)?;
    
    let geometry_filter = Arc::new( {
        if filter_str.is_none() {
            GeometryFilter::Null
        } else if let Some(pp) = poly {
            GeometryFilter::Poly(pp)
        } else {
            GeometryFilter::Bbox(bx.clone())
        }
    });
    let max_depth = match max_minzoom {
        None=>None,
        Some(md) => Some(md as usize)
    };
    
    let (mut files, locs, total_len)  = get_file_locs_max_depth(infn, Some(bx), None, max_depth)?;
    
    
    
    let r = if numchan == 0 {
        let cc = Box::new(CallAll::new(
            cb, "UnpackGeometry", 
            Box::new(move |(i,fb): (usize,Vec<FileBlock>)| {
                unpack_geometry_block_filter(i as i64, &fb[0].data(), &geometry_filter, &max_minzoom).unwrap()
            })
        ));
        
        Ok(read_all_blocks_parallel_with_progbar(&mut files, &locs, cc, "read geometry blocks", total_len))
    } else {
        Err(Error::NotImplementedError)
    }?;
    
    message!("{}", r);
    
    Ok(r)
}
    
        
        
        
