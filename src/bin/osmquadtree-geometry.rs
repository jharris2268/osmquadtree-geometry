extern crate clap;

use clap::{value_t, App, AppSettings, Arg, SubCommand};

//use osmquadtree::utils::{parse_timestamp, LogTimes};

use osmquadtree_geometry::postgresql::{PostgresqlConnection, PostgresqlOptions,prepare_tables};
use osmquadtree_geometry::{GeometryStyle, OutputType};

use osmquadtree::message;
use osmquadtree::defaultlogger::register_messenger_default;

use std::io::{Error, ErrorKind, Result};
//use std::sync::Arc;

fn process_geometry(
    prfx: &str,
    outfn: OutputType,
    filter: Option<&str>,
    timestamp: Option<&str>,
    find_minzoom: bool,
    style_name: Option<&str>,
    max_minzoom: Option<i64>,
    numchan: usize,
) -> Result<()> {
    osmquadtree_geometry::process_geometry(prfx, outfn, filter, timestamp, find_minzoom, style_name, max_minzoom, numchan)?;
    Ok(())
}


fn dump_geometry_style(outfn: Option<&str>) -> Result<()> {
    let outfn = match outfn {
        Some(o) => String::from(o),
        None => String::from("default_style.json"),
    };
    let mut f = std::fs::File::create(&outfn)?;
    serde_json::to_writer_pretty(&mut f, &GeometryStyle::default())?;
    Ok(())
}

fn get_i64(x: Option<&str>) -> Option<i64> {
    match x {
        None => None,
        Some(t) => Some(t.parse().expect("expected integer argument")),
    }
}

const NUMCHAN_DEFAULT: usize = 4;
/*const RAM_GB_DEFAULT: usize= 8;
const QT_MAX_LEVEL_DEFAULT: usize = 18;
const QT_GRAPH_LEVEL_DEFAULT: usize = 17;
const QT_BUFFER_DEFAULT: f64 = 0.05;
*/


fn main() {
    // basic app information
    register_messenger_default().expect("!!");
    
    
    let app = App::new("osmquadtree-geometry")
        .version("0.1")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author("James Harris")
        
        .subcommand(
            SubCommand::with_name("process_geometry_null")
                .about("process_geometry")
                .arg(Arg::with_name("INPUT").required(true).help("Sets the input directory to use"))
                .arg(Arg::allow_hyphen_values(Arg::with_name("FILTER").short("-f").long("--filter").takes_value(true).help("filters blocks by bbox FILTER"),true))
                .arg(Arg::with_name("TIMESTAMP").short("-t").long("--timestamp").takes_value(true).help("timestamp for data"))
                .arg(Arg::with_name("FIND_MINZOOM").short("-m").long("--minzoom").help("find minzoom"))
                .arg(Arg::with_name("STYLE_NAME").short("-s").long("--style").takes_value(true).help("style json filename"))
                .arg(Arg::with_name("NUMCHAN").short("-n").long("--numchan").takes_value(true).help("uses NUMCHAN parallel threads"))
        )
        .subcommand(
            SubCommand::with_name("process_geometry_json")
                .about("process_geometry")
                .arg(Arg::with_name("INPUT").required(true).help("Sets the input directory to use"))
                .arg(Arg::with_name("OUTFN").short("-o").long("--outfn").required(true).takes_value(true).help("out filename, "))
                .arg(Arg::allow_hyphen_values(Arg::with_name("FILTER").short("-f").long("--filter").takes_value(true).help("filters blocks by bbox FILTER"),true))
                .arg(Arg::with_name("TIMESTAMP").short("-t").long("--timestamp").takes_value(true).help("timestamp for data"))
                .arg(Arg::with_name("FIND_MINZOOM").short("-m").long("--minzoom").help("find minzoom"))
                .arg(Arg::with_name("STYLE_NAME").short("-s").long("--style").takes_value(true).help("style json filename"))
                .arg(Arg::with_name("MAX_MINZOOM").short("-M").long("--maxminzoom").takes_value(true).help("maximum minzoom value"))
                .arg(Arg::with_name("NUMCHAN").short("-n").long("--numchan").takes_value(true).help("uses NUMCHAN parallel threads"))
        )
        .subcommand(
            SubCommand::with_name("process_geometry_tiled_json")
                .about("process_geometry")
                .arg(Arg::with_name("INPUT").required(true).help("Sets the input directory to use"))
                .arg(Arg::with_name("OUTFN").short("-o").long("--outfn").required(true).takes_value(true).help("out filename, "))
                .arg(Arg::allow_hyphen_values(Arg::with_name("FILTER").short("-f").long("--filter").takes_value(true).help("filters blocks by bbox FILTER"),true))
                .arg(Arg::with_name("TIMESTAMP").short("-t").long("--timestamp").takes_value(true).help("timestamp for data"))
                .arg(Arg::with_name("FIND_MINZOOM").short("-m").long("--minzoom").help("find minzoom"))
                .arg(Arg::with_name("STYLE_NAME").short("-s").long("--style").takes_value(true).help("style json filename"))
                .arg(Arg::with_name("MAX_MINZOOM").short("-M").long("--maxminzoom").takes_value(true).help("maximum minzoom value"))
                .arg(Arg::with_name("NUMCHAN").short("-n").long("--numchan").takes_value(true).help("uses NUMCHAN parallel threads"))
        )
        .subcommand(
            SubCommand::with_name("process_geometry_pbffile")
                .about("process_geometry")
                .arg(Arg::with_name("INPUT").required(true).help("Sets the input directory to use"))
                .arg(Arg::with_name("OUTFN").short("-o").long("--outfn").required(true).takes_value(true).help("out filename, "))
                .arg(Arg::allow_hyphen_values(Arg::with_name("FILTER").short("-f").long("--filter").takes_value(true).help("filters blocks by bbox FILTER"),true))
                .arg(Arg::with_name("TIMESTAMP").short("-t").long("--timestamp").takes_value(true).help("timestamp for data"))
                .arg(Arg::with_name("FIND_MINZOOM").short("-m").long("--minzoom").help("find minzoom"))
                .arg(Arg::with_name("STYLE_NAME").short("-s").long("--style").takes_value(true).help("style json filename"))
                .arg(Arg::with_name("MAX_MINZOOM").short("-M").long("--maxminzoom").takes_value(true).help("maximum minzoom value"))
                .arg(Arg::with_name("SORT").short("-S").long("--short").help("sort out pbffile"))
                .arg(Arg::with_name("NUMCHAN").short("-n").long("--numchan").takes_value(true).help("uses NUMCHAN parallel threads"))
        )
        .subcommand(
            SubCommand::with_name("process_geometry_postgresqlnull")
                .about("process_geometry")
                .arg(Arg::with_name("INPUT").required(true).help("Sets the input directory to use"))
                .arg(Arg::allow_hyphen_values(Arg::with_name("FILTER").short("-f").long("--filter").takes_value(true).help("filters blocks by bbox FILTER"),true))
                .arg(Arg::with_name("TIMESTAMP").short("-t").long("--timestamp").takes_value(true).help("timestamp for data"))
                .arg(Arg::with_name("FIND_MINZOOM").short("-m").long("--minzoom").help("find minzoom"))
                .arg(Arg::with_name("STYLE_NAME").short("-s").long("--style").takes_value(true).help("style json filename"))
                .arg(Arg::with_name("EXTENDED").short("-e").long("--extended").help("extended table spec"))
                .arg(Arg::with_name("MAX_MINZOOM").short("-M").long("--maxminzoom").takes_value(true).help("maximum minzoom value"))
                .arg(Arg::with_name("NUMCHAN").short("-n").long("--numchan").takes_value(true).help("uses NUMCHAN parallel threads"))
        )
        .subcommand(
            SubCommand::with_name("process_geometry_postgresqlblob")
                .about("process_geometry")
                .arg(Arg::with_name("INPUT").required(true).help("Sets the input directory to use"))
                .arg(Arg::with_name("OUTFN").short("-o").long("--outfn").required(true).takes_value(true).help("out filename, "))
                .arg(Arg::allow_hyphen_values(Arg::with_name("FILTER").short("-f").long("--filter").takes_value(true).help("filters blocks by bbox FILTER"),true))
                .arg(Arg::with_name("TIMESTAMP").short("-t").long("--timestamp").takes_value(true).help("timestamp for data"))
                .arg(Arg::with_name("FIND_MINZOOM").short("-m").long("--minzoom").help("find minzoom"))
                .arg(Arg::with_name("STYLE_NAME").short("-s").long("--style").takes_value(true).help("style json filename"))
                .arg(Arg::with_name("EXTENDED").short("-e").long("--extended").help("extended table spec"))
                .arg(Arg::with_name("MAX_MINZOOM").short("-M").long("--maxminzoom").takes_value(true).help("maximum minzoom value"))
                .arg(Arg::with_name("NUMCHAN").short("-n").long("--numchan").takes_value(true).help("uses NUMCHAN parallel threads"))
        )
        .subcommand(
            SubCommand::with_name("process_geometry_postgresqlblob_pbf")
                .about("process_geometry")
                .arg(Arg::with_name("INPUT").required(true).help("Sets the input directory to use"))
                .arg(Arg::with_name("OUTFN").short("-o").long("--outfn").required(true).takes_value(true).help("out filename, "))
                .arg(Arg::allow_hyphen_values(Arg::with_name("FILTER").short("-f").long("--filter").takes_value(true).help("filters blocks by bbox FILTER"),true))
                .arg(Arg::with_name("TIMESTAMP").short("-t").long("--timestamp").takes_value(true).help("timestamp for data"))
                .arg(Arg::with_name("FIND_MINZOOM").short("-m").long("--minzoom").help("find minzoom"))
                .arg(Arg::with_name("STYLE_NAME").short("-s").long("--style").takes_value(true).help("style json filename"))
                .arg(Arg::with_name("EXTENDED").short("-e").long("--extended").help("extended table spec"))
                .arg(Arg::with_name("MAX_MINZOOM").short("-M").long("--maxminzoom").takes_value(true).help("maximum minzoom value"))
                .arg(Arg::with_name("NUMCHAN").short("-n").long("--numchan").takes_value(true).help("uses NUMCHAN parallel threads"))
        )
        .subcommand(
            SubCommand::with_name("process_geometry_postgresql")
                .about("process_geometry")
                .arg(Arg::with_name("INPUT").required(true).help("Sets the input directory to use"))
                .arg(Arg::with_name("CONNECTION").short("-c").long("--connection").required(true).takes_value(true).help("connection string"))
                .arg(Arg::with_name("TABLE_PREFIX").short("-p").long("--tableprefix").required(true).takes_value(true).help("table prfx"))
                .arg(Arg::allow_hyphen_values(Arg::with_name("FILTER").short("-f").long("--filter").takes_value(true).help("filters blocks by bbox FILTER"),true))
                .arg(Arg::with_name("TIMESTAMP").short("-t").long("--timestamp").takes_value(true).help("timestamp for data"))
                .arg(Arg::with_name("FIND_MINZOOM").short("-m").long("--minzoom").help("find minzoom"))
                .arg(Arg::with_name("STYLE_NAME").short("-s").long("--style").takes_value(true).help("style json filename"))
                .arg(Arg::with_name("EXTENDED").short("-e").long("--extended").help("extended table spec"))
                .arg(Arg::with_name("EXEC_INDICES").short("-I").long("--exec_inidices").help("execute indices [can be very slow for planet imports]"))
                .arg(Arg::with_name("MAX_MINZOOM").short("-M").long("--maxminzoom").takes_value(true).help("maximum minzoom value"))
                .arg(Arg::with_name("NUMCHAN").short("-n").long("--numchan").takes_value(true).help("uses NUMCHAN parallel threads"))
        )
        .subcommand(
            SubCommand::with_name("dump_geometry_style")
                .arg(Arg::with_name("OUTPUT").required(true))
        )
        .subcommand(
            SubCommand::with_name("show_after_queries")
                .arg(Arg::with_name("TABLE_PREFIX").short("-p").long("--tableprefix").takes_value(true).help("table prfx"))
                .arg(Arg::with_name("EXTENDED").short("-e").long("--extended").help("extended table spec"))
        )
        ;

    let mut help = Vec::new();
    app.write_help(&mut help).expect("?");

    let res = match app.get_matches().subcommand() {
        
        ("process_geometry_null", Some(geom)) => process_geometry(
            geom.value_of("INPUT").unwrap(),
            OutputType::None,
            geom.value_of("FILTER"),
            geom.value_of("TIMESTAMP"),
            geom.is_present("FIND_MINZOOM"),
            geom.value_of("STYLE_NAME"),
            get_i64(geom.value_of("MAX_MINZOOM")),
            value_t!(geom, "NUMCHAN", usize).unwrap_or(NUMCHAN_DEFAULT),
        ),
        ("process_geometry_json", Some(geom)) => process_geometry(
            geom.value_of("INPUT").unwrap(),
            OutputType::Json(String::from(geom.value_of("OUTFN").unwrap())),
            geom.value_of("FILTER"),
            geom.value_of("TIMESTAMP"),
            geom.is_present("FIND_MINZOOM"),
            geom.value_of("STYLE_NAME"),
            get_i64(geom.value_of("MAX_MINZOOM")),
            value_t!(geom, "NUMCHAN", usize).unwrap_or(NUMCHAN_DEFAULT),
        ),
        ("process_geometry_tiled_json", Some(geom)) => process_geometry(
            geom.value_of("INPUT").unwrap(),
            OutputType::TiledJson(String::from(geom.value_of("OUTFN").unwrap())),
            geom.value_of("FILTER"),
            geom.value_of("TIMESTAMP"),
            geom.is_present("FIND_MINZOOM"),
            geom.value_of("STYLE_NAME"),
            get_i64(geom.value_of("MAX_MINZOOM")),
            value_t!(geom, "NUMCHAN", usize).unwrap_or(NUMCHAN_DEFAULT),
        ),
        ("process_geometry_pbffile", Some(geom)) => {
            
            let ot = if geom.is_present("SORT") {
                OutputType::PbfFileSorted(String::from(geom.value_of("OUTFN").unwrap()))
            } else {
                OutputType::PbfFile(String::from(geom.value_of("OUTFN").unwrap()))
            };
            
            process_geometry(
                geom.value_of("INPUT").unwrap(),
                ot,
                geom.value_of("FILTER"),
                geom.value_of("TIMESTAMP"),
                geom.is_present("FIND_MINZOOM"),
                geom.value_of("STYLE_NAME"),
                get_i64(geom.value_of("MAX_MINZOOM")),
                value_t!(geom, "NUMCHAN", usize).unwrap_or(NUMCHAN_DEFAULT),
            )
        },
        ("process_geometry_postgresqlnull", Some(geom)) => {
            let pc = PostgresqlConnection::Null;
            let po = if geom.is_present("EXTENDED") {
                PostgresqlOptions::extended(pc, &GeometryStyle::default())
            } else {
                PostgresqlOptions::osm2pgsql(pc, &GeometryStyle::default())
            };
            process_geometry(
                geom.value_of("INPUT").unwrap(),
                OutputType::Postgresql(po),
                geom.value_of("FILTER"),
                geom.value_of("TIMESTAMP"),
                geom.is_present("FIND_MINZOOM"),
                geom.value_of("STYLE_NAME"),
                get_i64(geom.value_of("MAX_MINZOOM")),
                value_t!(geom, "NUMCHAN", usize).unwrap_or(NUMCHAN_DEFAULT),
            )
        }
        ("process_geometry_postgresqlblob", Some(geom)) => {
            let pc =
                PostgresqlConnection::CopyFilePrfx(String::from(geom.value_of("OUTFN").unwrap()));
            let po = if geom.is_present("EXTENDED") {
                PostgresqlOptions::extended(pc, &GeometryStyle::default())
            } else {
                PostgresqlOptions::osm2pgsql(pc, &GeometryStyle::default())
            };
            process_geometry(
                geom.value_of("INPUT").unwrap(),
                OutputType::Postgresql(po),
                geom.value_of("FILTER"),
                geom.value_of("TIMESTAMP"),
                geom.is_present("FIND_MINZOOM"),
                geom.value_of("STYLE_NAME"),
                get_i64(geom.value_of("MAX_MINZOOM")),
                value_t!(geom, "NUMCHAN", usize).unwrap_or(NUMCHAN_DEFAULT),
            )
        }
        ("process_geometry_postgresqlblob_pbf", Some(geom)) => {
            let pc =
                PostgresqlConnection::CopyFileBlob(String::from(geom.value_of("OUTFN").unwrap()));
            let po = if geom.is_present("EXTENDED") {
                PostgresqlOptions::extended(pc, &GeometryStyle::default())
            } else {
                PostgresqlOptions::osm2pgsql(pc, &GeometryStyle::default())
            };
            process_geometry(
                geom.value_of("INPUT").unwrap(),
                OutputType::Postgresql(po),
                geom.value_of("FILTER"),
                geom.value_of("TIMESTAMP"),
                geom.is_present("FIND_MINZOOM"),
                geom.value_of("STYLE_NAME"),
                get_i64(geom.value_of("MAX_MINZOOM")),
                value_t!(geom, "NUMCHAN", usize).unwrap_or(NUMCHAN_DEFAULT),
            )
        }
        ("process_geometry_postgresql", Some(geom)) => {
            let pc = PostgresqlConnection::Connection((
                String::from(geom.value_of("CONNECTION").unwrap()),
                String::from(geom.value_of("TABLE_PREFIX").unwrap()),
                geom.is_present("EXEC_INDICES"),
            ));
            let po = if geom.is_present("EXTENDED") {
                PostgresqlOptions::extended(pc, &GeometryStyle::default())
            } else {
                PostgresqlOptions::osm2pgsql(pc, &GeometryStyle::default())
            };
            process_geometry(
                geom.value_of("INPUT").unwrap(),
                OutputType::Postgresql(po),
                geom.value_of("FILTER"),
                geom.value_of("TIMESTAMP"),
                geom.is_present("FIND_MINZOOM"),
                geom.value_of("STYLE_NAME"),
                get_i64(geom.value_of("MAX_MINZOOM")),
                value_t!(geom, "NUMCHAN", usize).unwrap_or(NUMCHAN_DEFAULT),
            )
        }
        ("dump_geometry_style", Some(geom)) => dump_geometry_style(geom.value_of("OUTPUT")),
        
        ("show_after_queries", Some(geom)) => {
            (|| {
                let pc = PostgresqlConnection::Connection((String::new(),String::new(),true));
                let po = if geom.is_present("EXTENDED") {
                    PostgresqlOptions::extended(pc, &GeometryStyle::default())
                } else {
                    PostgresqlOptions::osm2pgsql(pc, &GeometryStyle::default())
                };
                let lz = if po.extended { Some(Vec::from([(String::from("lz6_"),6,true),(String::from("lz9_"),9,false),(String::from("lz11_"),11,false)])) } else {None};
                message!("{}", prepare_tables(geom.value_of("TABLE_PREFIX"), 
                    &po.table_spec, 
                    po.extended,
                    po.extended,
                    &lz)?.2.join("\n"));
                Ok(())
            })()
        },
        
        _ => Err(Error::new(ErrorKind::Other, "??")),
        
    };

    match res {
        Ok(()) => {}
        Err(err) => {
            message!("FAILED: {}", err);
            message!("{}", String::from_utf8(help).unwrap());
        }
    }

    //message!("count: {:?}", matches);
}
