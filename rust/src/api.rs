use crate::solver::{Config, base_solver, generate_palette};
use num_bigint::BigUint;
use rayon::prelude::*;
use std::sync::Mutex;
use lru::LruCache;
use std::num::NonZeroUsize;
use num_traits::ToPrimitive;
use rhai::Engine;

lazy_static::lazy_static! {
    static ref CACHE: Mutex<LruCache<String, BigUint>> = Mutex::new(LruCache::new(NonZeroUsize::new(500000).unwrap()));
}

// Wrapper to handle LRU caching and Post Processing Math
fn get_post_processed_val(n: &BigUint, cfg: &Config, engine: &Engine, l_ast: &Option<rhai::AST>, r1_ast: &Option<rhai::AST>, r2_ast: &Option<rhai::AST>) -> BigUint {
    let big_m = BigUint::from(cfg.m);
    
    let cached_solve = |num: &BigUint| -> BigUint {
        let key = format!("{}_{}", num.to_str_radix(10), cfg.b);
        let mut cache = CACHE.lock().unwrap();
        if let Some(res) = cache.get(&key) { return res.clone(); }
        let res = base_solver(num, cfg, engine, l_ast, r1_ast, r2_ast);
        cache.put(key, res.clone());
        res
    };

    if cfg.post_type == "NONE" {
        cached_solve(n) % &big_m
    } else if cfg.post_type == "ITERATE" {
        let mut val = n.clone();
        for _ in 0..cfg.post_k { val = cached_solve(&val); }
        val % &big_m
    } else if cfg.post_type == "C_SEQ" {
        let block_size = BigUint::from(cfg.grid_r * cfg.grid_c);
        let block = n / &block_size;
        let idx = (n % &block_size).to_usize().unwrap();
        let r = idx / cfg.grid_c; let c = idx % cfg.grid_c;
        let mut count = 0u32;
        
        for dr in -1..=1 {
            for dc in -1..=1 {
                if dr == 0 && dc == 0 { continue; }
                let nr = r as i32 + dr; let nc = c as i32 + dc;
                if nr >= 0 && nr < cfg.grid_r as i32 && nc >= 0 && nc < cfg.grid_c as i32 {
                    let n_prime = &block * &block_size + BigUint::from(nr as usize * cfg.grid_c + nc as usize);
                    let val = cached_solve(&n_prime);
                    if (&val % BigUint::from(cfg.mod_mc)).to_u32().unwrap() == cfg.target_t { count += 1; }
                }
            }
        }
        BigUint::from(count) % &big_m
    } else { // D_SEQ
        let block_size = BigUint::from(cfg.grid_r * cfg.grid_c);
        let base_n = n * &block_size;
        let min_dim = std::cmp::min(cfg.grid_r, cfg.grid_c);
        let mut sum = BigUint::zero();
        for i in 0..min_dim {
            let n_prime = &base_n + BigUint::from(i * cfg.grid_c + i);
            sum += cached_solve(&n_prime);
        }
        sum % &big_m
    }
}

// ENDPOINT 1: Interactive Viewer / High Res Exporter
pub fn compile_grid(cfg: Config) -> Vec<u8> {
    let engine = Engine::new();
    let l_ast = if cfg.lhs == 13 { engine.compile(&cfg.custom_lhs).ok() } else { None };
    let r1_ast = if cfg.rhs1 == 12 { engine.compile(&cfg.custom_rhs1).ok() } else { None };
    let r2_ast = if cfg.rhs2 == 12 { engine.compile(&cfg.custom_rhs2).ok() } else { None };

    let start = BigUint::parse_bytes(cfg.start_n.as_bytes(), 10).unwrap();
    let palette = generate_palette(cfg.m);
    
    (0..(cfg.render_r * cfg.render_c)).into_par_iter().flat_map(|i| {
        let n = &start + BigUint::from(i);
        let mod_val = get_post_processed_val(&n, &cfg, &engine, &l_ast, &r1_ast, &r2_ast).to_usize().unwrap();
        let color = palette.get(mod_val).unwrap_or(&[0,0,0]);
        vec![color[0], color[1], color[2], 255]
    }).collect()
}

// ENDPOINT 2: GIF Modes (A, B, C)
pub fn compile_gif_animation(mut cfg: Config, mode: String, frames_start: u32, frames_end: u32) -> Vec<u8> {
    let width = cfg.render_c as u16; let height = cfg.render_r as u16;
    let mut gif_data = Vec::new();
    let mut encoder = gif::Encoder::new(&mut gif_data, width, height, &[]).unwrap();
    encoder.set_repeat(gif::Repeat::Infinite).unwrap();
    
    let engine = Engine::new();
    let l_ast = if cfg.lhs == 13 { engine.compile(&cfg.custom_lhs).ok() } else { None };
    let r1_ast = if cfg.rhs1 == 12 { engine.compile(&cfg.custom_rhs1).ok() } else { None };
    let r2_ast = if cfg.rhs2 == 12 { engine.compile(&cfg.custom_rhs2).ok() } else { None };

    let total_frames = if mode == "modeA" { cfg.b } else { frames_end - frames_start + 1 };
    
    for step in 0..total_frames {
        let mut cur_pal = generate_palette(cfg.m);
        
        if mode == "modeB" { cfg.b = frames_start + step; }
        if mode == "modeC" { 
            cfg.m = frames_start + step; 
            cur_pal = generate_palette(cfg.m); 
        }
        
        let mut flat_palette = Vec::new();
        for col in &cur_pal { flat_palette.extend_from_slice(col); }

        let base_offset = BigUint::parse_bytes(cfg.p.as_bytes(), 10).unwrap();
        let n_start = if mode == "modeA" {
            (base_offset * cfg.b + step) * (cfg.render_r * cfg.render_c)
        } else {
            base_offset * (cfg.render_r * cfg.render_c)
        };
        
        let index_buffer: Vec<u8> = (0..(cfg.render_r * cfg.render_c)).into_par_iter().map(|i| {
            let n = &n_start + BigUint::from(i);
            let val = get_post_processed_val(&n, &cfg, &engine, &l_ast, &r1_ast, &r2_ast);
            val.to_u8().unwrap_or(0)
        }).collect();
        
        let mut frame = gif::Frame::from_indexed_pixels(width, height, &index_buffer, Some(&flat_palette));
        frame.delay = 50; 
        encoder.write_frame(&frame).unwrap();
    }
    gif_data
}
