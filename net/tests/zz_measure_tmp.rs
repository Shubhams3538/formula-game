use racecraft_net::*;
use racecraft_sim::{Assists, CarInput, Driver, PHYSICS_RATE};
use std::{
    net::{SocketAddr, UdpSocket},
    path::PathBuf,
    time::{Duration, Instant},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("racecraft-net-test-{}", random_id()));
        std::fs::create_dir_all(root.join("tracks/spa")).unwrap();
        std::fs::create_dir_all(root.join("cars/rb6")).unwrap();
        let points: Vec<_> = (0..64)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 64.;
                [150. * a.cos(), 0., 150. * a.sin()]
            })
            .collect();
        let track = serde_json::json!({"name":"Network oval","source":"test","centerline":points,"half_width":vec![12.;64]});
        std::fs::write(
            root.join("tracks/spa/track.json"),
            serde_json::to_vec(&track).unwrap(),
        )
        .unwrap();
        let mut bytes = b"F1CL".to_vec();
        for n in [1u32, 2, 0] {
            bytes.extend(n.to_le_bytes());
        }
        for vertex in [
            [-500f32, 0., -500.],
            [-500., 0., 500.],
            [500., 0., -500.],
            [500., 0., -500.],
            [-500., 0., 500.],
            [500., 0., 500.],
        ] {
            for x in vertex {
                bytes.extend(x.to_le_bytes());
            }
        }
        bytes.extend([0u8; 2]);
        std::fs::write(root.join("tracks/spa/collision.bin"), bytes).unwrap();
        std::fs::write(
            root.join("cars/rb6/car.json"),
            include_bytes!("fixtures/car.json"),
        )
        .unwrap();
        Self(root)
    }
}
#[test]
fn measure() {
    let f = Fixture::new();
    let players = vec![Player { id: 1, name: "P".into(), ready: true, connected: false, identity: None, assists: Assists::default() }];
    let settings = RaceSettings { ai_cars: 15, ..RaceSettings::default() };
    let mut race = load_race(&f.0, &settings, &players).unwrap();
    let mut hist: Vec<Vec<u8>> = vec![];
    let opts = || { use bincode::Options; bincode::DefaultOptions::new().with_fixint_encoding() };
    for t in 0..(360*40) {
        race.step(&[CarInput::default(); 16]);
        if t % 12 == 0 { use bincode::Options; hist.push(opts().serialize(&race.snapshot()).unwrap()); }
    }
    let n = hist.len();
    let cur = &hist[n-1];
    let full = miniz_oxide::deflate::compress_to_vec_zlib(cur, 1);
    let full6 = miniz_oxide::deflate::compress_to_vec_zlib(cur, 6);
    println!("raw={} full_l1={} full_l6={}", cur.len(), full.len(), full6.len());
    for back in [1usize, 2, 4, 8] {
        let base = &hist[n-1-back];
        let x: Vec<u8> = cur.iter().enumerate().map(|(i,b)| b ^ base.get(i).copied().unwrap_or(0)).collect();
        println!("back={back} samelen={} xor_l1={} xor_l6={}", base.len()==cur.len(), miniz_oxide::deflate::compress_to_vec_zlib(&x,1).len(), miniz_oxide::deflate::compress_to_vec_zlib(&x,6).len());
    }
    
    fn nums(v:&serde_json::Value,out:&mut Vec<f64>,ints:&mut usize){match v{serde_json::Value::Number(n)=>{if n.is_f64(){out.push(n.as_f64().unwrap())}else{*ints+=1}},serde_json::Value::Array(a)=>a.iter().for_each(|x|nums(x,out,ints)),serde_json::Value::Object(o)=>o.values().for_each(|x|nums(x,out,ints)),_=>{}}}
    let snaps: Vec<_> = { let mut r = load_race(&f.0,&settings,&players).unwrap(); let mut v=vec![]; for t in 0..(360*40){ r.step(&[CarInput::default();16]); if t>=360*40-24 && t%12==0 {v.push(serde_json::to_value(r.snapshot()).unwrap())}} v };
    let (mut a, mut b, mut ia)=(vec![],vec![],0); let mut ib=0; nums(&snaps[1],&mut a,&mut ia); nums(&snaps[0],&mut b,&mut ib);
    let f64b:Vec<u8>=a.iter().flat_map(|x|x.to_le_bytes()).collect();
    let f32b:Vec<u8>=a.iter().flat_map(|x|(*x as f32).to_le_bytes()).collect();
    let f32x:Vec<u8>=a.iter().zip(&b).flat_map(|(x,y)|((*x as f32).to_bits()^(*y as f32).to_bits()).to_le_bytes()).collect();
    let unchanged=a.iter().zip(&b).filter(|(x,y)|x==y).count();
    let c=|v:&[u8]|miniz_oxide::deflate::compress_to_vec_zlib(v,6).len();
    println!("floats={} ints={} unchanged_vs_prev={} f64={}->{} f32={}->{} f32xor->{}", a.len(), ia, unchanged, f64b.len(), c(&f64b), f32b.len(), c(&f32b), c(&f32x));
    println!("phase={:?} tick={}", race.phase, race.tick);
}
