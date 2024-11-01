pub use crate::postgresql::{make_write_postgresql_geometry, PostgresqlOptions};

use crate::addparenttag::AddParentTag;
use crate::elements::{GeoJsonable,WithBounds};
use crate::minzoom::{FindMinZoom, MinZoomSpec};
use crate::multipolygons::ProcessMultiPolygons;

use crate::position::{calc_line_length, calc_ring_area};
use crate::relationtags::AddRelationTags;
use crate::{
    CollectWayNodes, GeometryBlock, GeometryStyle, LinestringGeometry, OtherData, PointGeometry,
    SimplePolygonGeometry, ComplicatedPolygonGeometry, Timings, WorkingBlock,CallFinishGeometryBlock,
    prep_write_geometry_pbffile, make_write_temp_geometry, write_temp_geometry
};

use crate::{Error, Result};

use channelled_callbacks::{CallFinish, Callback, CallbackMerge, CallbackSync, MergeTimings, ReplaceNoneWithTimings, Result as ccResult};
use osmquadtree::utils::{
    parse_timestamp, LogTimes, ThreadTimer,
};
use osmquadtree::message;
use osmquadtree::elements::{Block, Quadtree};
use osmquadtree::mergechanges::read_filter;

use osmquadtree::pbfformat::{
    make_read_primitive_blocks_combine_call_all,
    //read_primitive_blocks_combine,
    read_all_blocks_parallel_with_progbar, FileBlock,
    ParallelFileLocs, get_file_locs
};
use osmquadtree::sortblocks::{TempData,QuadtreeTree};



use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;


pub struct StoreBlocks {
    tiles: BTreeMap<Quadtree, GeometryBlock>,
    rem: Option<GeometryBlock>,
    nt: usize,
}

impl StoreBlocks {
    pub fn new(qq: Vec<Quadtree>) -> StoreBlocks {
        let mut tiles = BTreeMap::new();
        for q in qq {
            tiles.insert(q, GeometryBlock::new(tiles.len() as i64, q.clone(), 0));
        }
        StoreBlocks {
            tiles: tiles, 
            nt: 0,
            rem: Some(GeometryBlock::new(-1, Quadtree::empty(), 0))
        }
    }
    fn add_point(&mut self, p: PointGeometry) {
        for i in (0..p.quadtree.depth()).rev() {
            match self.tiles.get_mut(&p.quadtree.round(i)) {
                Some(b) => { b.points.push(p); return; }
                _ => {}
            }
        }
        self.rem.as_mut().unwrap().points.push(p);
        //return self.tiles.get_mut(&Quadtree::new(0)).unwrap()
    }

    fn add_linestring(&mut self, p: LinestringGeometry) {
        for i in (0..p.quadtree.depth()).rev() {
            match self.tiles.get_mut(&p.quadtree.round(i)) {
                Some(b) => { b.linestrings.push(p); return; }
                _ => {}
            }
        }
        self.rem.as_mut().unwrap().linestrings.push(p);
        //return self.tiles.get_mut(&Quadtree::new(0)).unwrap()
    }
    fn add_simple_polygon(&mut self, p: SimplePolygonGeometry) {
        for i in (0..p.quadtree.depth()).rev() {
            match self.tiles.get_mut(&p.quadtree.round(i)) {
                Some(b) => { b.simple_polygons.push(p); return; }
                _ => {}
            }
        }
        self.rem.as_mut().unwrap().simple_polygons.push(p);
        //return self.tiles.get_mut(&Quadtree::new(0)).unwrap()
    }
    fn add_complicated_polygon(&mut self, p: ComplicatedPolygonGeometry) {
        for i in (0..p.quadtree.depth()).rev() {
            match self.tiles.get_mut(&p.quadtree.round(i)) {
                Some(b) => { b.complicated_polygons.push(p); return; }
                _ => {}
            }
        }
        self.rem.as_mut().unwrap().complicated_polygons.push(p);
        //return self.tiles.get_mut(&Quadtree::new(0)).unwrap()
    }
}

impl CallFinish for StoreBlocks {
    type CallType = GeometryBlock;
    type ReturnType = Timings;
    type ErrorType = Error;
    
            

    fn call(&mut self, gb: GeometryBlock) {
        self.nt += 1;
        if gb.len() == 0 {
            return;
        }
        
        
        for p in gb.points {
            self.add_point(p);
        }
        for p in gb.linestrings {
            self.add_linestring(p);
        }
        for p in gb.simple_polygons {
            self.add_simple_polygon(p);
        }
        for p in gb.complicated_polygons {
            self.add_complicated_polygon(p);
        }
    }

    fn finish(&mut self) -> ccResult<Timings, Error> {
        let mut tms = Timings::new();
        let rem = std::mem::take(&mut self.rem).unwrap();
        if rem.len()>0 {
            self.tiles.insert(Quadtree::empty(), rem);
        }
        for (_, t) in self.tiles.iter_mut() {
            t.sort();
        }
        
        
        tms.add_other(
            "StoreBlocks",
            OtherData::Messages(vec![format!(
                "recieved {} blocks, returning {} blocks",
                self.nt,
                self.tiles.len()
            )]),
        );
        tms.add_other(
            "StoreBlocks",
            OtherData::GeometryBlocks(std::mem::take(&mut self.tiles)),
        );
        Ok(tms)
    }
}

struct CollectWorkingTiles<T: ?Sized> {
    npt: usize,
    nls: usize,
    nsp: usize,
    ncp: usize,
    out: Option<Box<T>>,
}

impl<T> CollectWorkingTiles<T>
where
    T: CallFinish<CallType = GeometryBlock, ReturnType = Timings> + ?Sized,
{
    pub fn new(out: Option<Box<T>>) -> CollectWorkingTiles<T> {
        CollectWorkingTiles {
            npt: 0,
            nls: 0,
            nsp: 0,
            ncp: 0,
            out: out,
        }
    }
}

impl<T> CallFinish for CollectWorkingTiles<T>
where
    T: CallFinish<CallType = GeometryBlock, ReturnType = Timings, ErrorType=Error> + ?Sized,
{
    type CallType = WorkingBlock;
    type ReturnType = Timings;
    type ErrorType = Error;

    fn call(&mut self, wb: WorkingBlock) {
        self.npt += wb.geometry_block.points.len();
        self.nls += wb.geometry_block.linestrings.len();
        self.nsp += wb.geometry_block.simple_polygons.len();
        self.ncp += wb.geometry_block.complicated_polygons.len();

        match self.out.as_mut() {
            None => {}
            Some(out) => {
                out.call(wb.geometry_block);
            }
        }
    }

    fn finish(&mut self) -> ccResult<Timings, Error> {
        let mut tms = match self.out.as_mut() {
            None => Timings::new(),
            Some(out) => out.finish()?,
        };

        let m = format!(
            "have {} points, {} linestrings, {} simple_polygons and {} complicated_polygons",
            self.npt, self.nls, self.nsp, self.ncp
        );
        tms.add_other("CollectWorkingTiles", OtherData::Messages(vec![m]));

        Ok(tms)
    }
}

struct MakeGeometries<T: ?Sized> {
    out: Box<T>,
    style: Arc<GeometryStyle>,
    recalcquadtree: bool,
    tm: f64,
    npt: usize,
    nls: usize,
    nsp: usize,
}

impl<T> MakeGeometries<T>
where
    T: CallFinish<CallType = WorkingBlock, ReturnType = Timings, ErrorType=Error> + ?Sized,
{
    pub fn new(out: Box<T>, style: Arc<GeometryStyle>, recalcquadtree: bool) -> MakeGeometries<T> {
        MakeGeometries {
            out: out,
            style: style,
            recalcquadtree: recalcquadtree,
            tm: 0.0,
            npt: 0,
            nls: 0,
            nsp: 0,
        }
    }

    fn process_block(&mut self, bl: &mut WorkingBlock) {
        std::mem::take(&mut bl.pending_relations);

        for n in std::mem::take(&mut bl.pending_nodes) {
            match self.style.process_node(&n.tags) {
                Err(_) => {}
                Ok((t, l)) => {
                    bl.geometry_block
                        .points
                        .push(PointGeometry::from_node(n, t, l));
                    self.npt += 1;
                }
            }
        }

        for (w, ll) in std::mem::take(&mut bl.pending_ways) {
            let is_ring = w.refs[0] == w.refs[w.refs.len() - 1];

            match self.style.process_way(&w.tags, is_ring) {
                Err(_) => {}
                Ok((is_poly, tgs, zorder, layer)) => {
                    if is_poly {
                        let area = calc_ring_area(&ll); //.iter().collect::<Vec<&LonLat>>());
                        let reversed = area < 0.0;
                        bl.geometry_block
                            .simple_polygons
                            .push(SimplePolygonGeometry::from_way(
                                w,
                                ll,
                                tgs,
                                f64::abs(area),
                                layer,
                                None,
                                reversed,
                            )); //no zorder for polys
                        self.nsp += 1;
                    } else {
                        let length = calc_line_length(&ll); //.iter().collect::<Vec<&LonLat>>());
                        bl.geometry_block
                            .linestrings
                            .push(LinestringGeometry::from_way(
                                w, ll, tgs, length, layer, zorder,
                            ));
                        self.nls += 1;
                    }
                }
            }
        }

        if self.recalcquadtree {
            for n in bl.geometry_block.points.iter_mut() {
                n.quadtree = Quadtree::calculate_point(n.lonlat.lon, n.lonlat.lat, 18, 0.0);
            }

            for l in bl.geometry_block.linestrings.iter_mut() {
                l.quadtree = Quadtree::calculate(&l.bounds(), 18, 0.0);
            }

            for sp in bl.geometry_block.simple_polygons.iter_mut() {
                sp.quadtree = Quadtree::calculate(&sp.bounds(), 18, 0.0);
            }

            for sp in bl.geometry_block.complicated_polygons.iter_mut() {
                let bnd = sp.bounds();
                sp.quadtree = Quadtree::calculate(&bnd, 18, 0.0);
            }
        }
    }
}

impl<T> CallFinish for MakeGeometries<T>
where
    T: CallFinish<CallType = WorkingBlock, ReturnType = Timings, ErrorType=Error> + ?Sized,
{
    type CallType = WorkingBlock;
    type ReturnType = Timings;
    type ErrorType = Error;

    fn call(&mut self, mut bl: WorkingBlock) {
        let tx = ThreadTimer::new();
        self.process_block(&mut bl);
        self.tm += tx.since();
        self.out.call(bl);
    }

    fn finish(&mut self) -> ccResult<Timings, Error> {
        let mut tms = self.out.finish()?;
        tms.add("MakeGeometries", self.tm);
        tms.add_other(
            "MakeGeometries",
            OtherData::Messages(vec![format!(
                "{} points, {} linestrings, {} simple polygons",
                self.npt, self.nls, self.nsp
            )]),
        );
        Ok(tms)
    }
}

fn write_geojson_tiles(tiles: &BTreeMap<Quadtree, GeometryBlock>, outfn: &str) -> Result<()> {
    let mut v = Vec::new();
    for (_, t) in tiles {
        v.push(t.to_geojson(false)?);
    }
    serde_json::to_writer(std::fs::File::create(outfn)?, &v)?;
    Ok(())
}

fn pack_feature_collection<F: GeoJsonable>(feats: &[F]) -> Result<Value> {
    let mut vv = Vec::with_capacity(feats.len());
    for f in feats {
        vv.push(f.to_geojson(false)?);
    }
    let mut m = Map::new();
    m.insert(String::from("type"), json!("FeatureCollection"));
    m.insert(String::from("features"), json!(vv));
    Ok(json!(m))
}

fn write_geojson_flat(tiles: BTreeMap<Quadtree, GeometryBlock>, outfn: &str) -> Result<()> {
    let mut tt = GeometryBlock::new(0, Quadtree::empty(), 0);
    for (_, t) in tiles {
        tt.extend(t);
    }
    tt.sort();

    let mut m = Map::new();
    m.insert(String::from("points"), pack_feature_collection(&tt.points)?);
    m.insert(
        String::from("linestrings"),
        pack_feature_collection(&tt.linestrings)?,
    );
    m.insert(
        String::from("simple_polygons"),
        pack_feature_collection(&tt.simple_polygons)?,
    );
    m.insert(
        String::from("complicated_polygons"),
        pack_feature_collection(&tt.complicated_polygons)?,
    );

    serde_json::to_writer(std::fs::File::create(outfn)?, &m)?;

    Ok(())
}

pub enum OutputType {
    None,
    Collect,
    Json(String),
    TiledJson(String),
    PbfFile(String),
    PbfFileSorted(String),
    Postgresql(PostgresqlOptions),
}


/*
fn wrap_read_primitive_blocks_combine(idx_blocks: (usize, Vec<FileBlock>)) -> PrimitiveBlock {
    read_primitive_blocks_combine(idx_blocks.0 as i64, idx_blocks.1, None)
        .expect("failed to read data")
}

fn make_read_primitive_blocks_combine_call_all(
    out: Box<CallFinish<CallType = PrimitiveBlock, ReturnType = Timings, ErrorType=Error>>,
) -> Box<impl CallFinish<CallType = (usize, Vec<FileBlock>), ReturnType = Timings, ErrorType=Error>> {
    Box::new(CallAll::new(
        out,
        "read_primitive_blocks_combine",
        Box::new(wrap_read_primitive_blocks_combine),
    ))
}
*/

pub fn process_geometry_call(
    pfilelocs: &mut ParallelFileLocs,
    out: Option<CallFinishGeometryBlock>,
    style: Arc<GeometryStyle>,
    minzoom: Option<MinZoomSpec>,
    numchan: usize,

) -> Timings {
    
    

    let cf = Box::new(CollectWorkingTiles::new(out));

    type CallFinishWorkingBlock =
        Box<dyn CallFinish<CallType = WorkingBlock, ReturnType = Timings, ErrorType=Error>>;

    let pp: Box<dyn CallFinish<CallType = (usize, Vec<FileBlock>), ReturnType = Timings, ErrorType=Error>> =
        if numchan == 0 {
            let fm: CallFinishWorkingBlock = if !minzoom.is_none() {
                Box::new(FindMinZoom::new(cf, minzoom))
            } else {
                cf
            };

            let mg = Box::new(MakeGeometries::new(fm, style.clone(), true));

            let mm: CallFinishWorkingBlock = if style.multipolygons || style.boundary_relations {
                Box::new(ProcessMultiPolygons::new(style.clone(), mg))
            } else {
                mg
            };

            let rt: CallFinishWorkingBlock = if !style.relation_tag_spec.is_empty() {
                Box::new(AddRelationTags::new(mm, style.clone()))
            } else {
                mm
            };

            let ap: CallFinishWorkingBlock = if !style.parent_tags.is_empty() {
                Box::new(AddParentTag::new(rt, style.clone()))
            } else {
                rt
            };

            let ww = Box::new(CollectWayNodes::new(ap, style.clone()));
            make_read_primitive_blocks_combine_call_all(ww)
        } else {
            let cfb = Box::new(Callback::new(cf));
            let fm: CallFinishWorkingBlock = if !minzoom.is_none() {
                Box::new(Callback::new(Box::new(FindMinZoom::new(cfb, minzoom))))
            } else {
                cfb
            };
            let mg = Box::new(Callback::new(Box::new(MakeGeometries::new(
                fm,
                style.clone(),
                true,
            ))));
            let mm: CallFinishWorkingBlock = if style.multipolygons || style.boundary_relations {
                Box::new(Callback::new(Box::new(ProcessMultiPolygons::new(
                    style.clone(),
                    mg,
                ))))
            } else {
                mg
            };
            let rt: CallFinishWorkingBlock = if !style.relation_tag_spec.is_empty() {
                Box::new(Callback::new(Box::new(AddRelationTags::new(
                    mm,
                    style.clone(),
                ))))
            } else {
                mm
            };
            let ap: CallFinishWorkingBlock = if !style.parent_tags.is_empty() {
                Box::new(Callback::new(Box::new(AddParentTag::new(
                    rt,
                    style.clone(),
                ))))
            } else {
                rt
            };

            let ww = CallbackSync::new(Box::new(CollectWayNodes::new(ap, style.clone())), numchan);

            let mut pps: Vec<
                Box<dyn CallFinish<CallType = (usize, Vec<FileBlock>), ReturnType = Timings, ErrorType=Error>>,
            > = Vec::new();
            for w in ww {
                let w2 = Box::new(ReplaceNoneWithTimings::new(w));
                pps.push(Box::new(Callback::new(
                    make_read_primitive_blocks_combine_call_all(w2),
                )))
            }
            Box::new(CallbackMerge::new(pps, Box::new(MergeTimings::new())))
        };

    let msg = format!("process_geometry, numchan={}", numchan);
    
    read_all_blocks_parallel_with_progbar(
        &mut pfilelocs.0,
        &pfilelocs.1,
        pp,
        &msg,
        pfilelocs.2,
    )
}


pub fn process_geometry(
    prfx: &str,
    outfn: OutputType,
    filter: Option<&str>,
    timestamp: Option<&str>,
    find_minzoom: bool,
    style_name: Option<&str>,
    max_minzoom: Option<i64>,
    numchan: usize,
) -> Result<Option<Vec<GeometryBlock>>> {
    let mut tx = LogTimes::new();
    let (bbox, poly) = read_filter(filter)?;

    message!("bbox={}, poly={:?}", bbox, poly);

    tx.add("read filter");
    let timestamp = match timestamp {
        None => None,
        Some(ts) => Some(parse_timestamp(ts)?),
    };

    let mut pfilelocs = get_file_locs(prfx, Some(bbox.clone()), timestamp)?;
    tx.add("get_file_locs");

    let style = match style_name {
        None => Arc::new(GeometryStyle::default()),
        Some(fname) => Arc::new(GeometryStyle::from_file(&fname)?),
    };
    tx.add("load_style");

    if !find_minzoom && !max_minzoom.is_none() {
        return Err(Error::UserSelectionError(format!("must run with find_minzoom=true if specifing max_minzoom")));
    }

    let minzoom: Option<MinZoomSpec> = if find_minzoom {
        message!("MinZoomSpec::default({}, {:?})", 5.0, max_minzoom);
        Some(MinZoomSpec::default(5.0, max_minzoom))
    } else {
        
        None
    };
    tx.add("load_minzoom");
    
    
    let mut groups: Option<Arc<QuadtreeTree>> = None;
    
    
    let out: Option<Box<dyn CallFinish<CallType = GeometryBlock, ReturnType = Timings, ErrorType=Error>>> =
        match &outfn {
            OutputType::None => None,
            OutputType::Collect | OutputType::Json(_) | OutputType::TiledJson(_) => {
                let mut qq = Vec::new();
                for a in &pfilelocs.1 {
                    qq.push(a.0.clone());
                }
                Some(Box::new(StoreBlocks::new(qq)))
            },
            OutputType::PbfFile(ofn) => {
                Some(prep_write_geometry_pbffile(ofn, &bbox, numchan)?)
            },
            OutputType::PbfFileSorted(ofn) => {
                let (pp,gg) = make_write_temp_geometry(ofn, &pfilelocs, &max_minzoom, numchan)?;
                groups = Some(gg);
                Some(pp)
            },
                
            OutputType::Postgresql(options) => {
                Some(make_write_postgresql_geometry(&options, numchan)?)
            }
        };
    
    let tm = process_geometry_call(&mut pfilelocs, out, style, minzoom, numchan);    

    tx.add("process_geometry");

    message!("{}", tm);
    let mut all_tiles = BTreeMap::new();
    let mut tempdata: Option<TempData> = None;
    for (w, x) in tm.others {
        match x {
            OtherData::Messages(mm) => {
                for m in mm {
                    message!("{}: {}", w, m);
                }
            }
            OtherData::Errors(ee) => {
                message!("{}: {} errors", w, ee.len());
            }
            OtherData::GeometryBlocks(tiles) => {
                all_tiles.extend(tiles);
            },
            OtherData::TempData(td) => { tempdata = Some(td); },
        }
    }
    tx.add("finish process_geometry");
    
    let out = match outfn {
        OutputType::None | OutputType::PbfFile(_) | OutputType::Postgresql(_) => { None },
        OutputType::Collect => Some(all_tiles.into_values().collect()),
        OutputType::PbfFileSorted(outfn) => {
            write_temp_geometry(&outfn, &bbox, tempdata.unwrap(), groups.unwrap(), numchan)?;
            tx.add("write final pbf");
            None
        },
        OutputType::Json(ofn) => {
            if !all_tiles.is_empty() {
                write_geojson_flat(all_tiles, &ofn)?;
                tx.add("write json");
            }
            None
        }
        OutputType::TiledJson(ofn) => {
            if !all_tiles.is_empty() {
                write_geojson_tiles(&all_tiles, &ofn)?;
                tx.add("write json");
            }
            Some(all_tiles.into_values().collect())
        }
    };
    

    message!("{}", tx);
    Ok(out)
        
}
