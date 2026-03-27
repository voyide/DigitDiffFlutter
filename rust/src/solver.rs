use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};
use rhai::{Engine, Scope, AST};

#[derive(Clone)]
pub struct Config {
    pub b: u32,
    pub p: String,
    pub m: u32,
    pub k_offset: i32,
    pub lhs: u32,
    pub rhs1: u32,
    pub rhs2: u32,
    pub logic: String,
    pub custom_lhs: String,
    pub custom_rhs1: String,
    pub custom_rhs2: String,
    pub post_type: String,
    pub post_k: u32,
    pub grid_r: usize,
    pub grid_c: usize,
    pub target_t: u32,
    pub mod_mc: u32,
    pub render_r: usize,
    pub render_c: usize,
    pub start_n: String,
}

fn get_d_array(n: &BigUint, b: u32, k_offset: i32) -> (Vec<u32>, usize) {
    let big_b = BigUint::from(b);
    let mut k = 0;
    if !n.is_zero() {
        let mut t = n.clone();
        while t > BigUint::zero() {
            k += 1;
            t /= &big_b;
        }
    } else {
        k = 1;
    }
    
    let l_end = std::cmp::max(1, k as i32 + k_offset - 1) as usize;
    let mut d = vec![0u32; l_end + 3];
    
    if !n.is_zero() {
        let mut temp = n.clone();
        let mut idx = k;
        while temp > BigUint::zero() && idx > 0 {
            d[idx] = (&temp % &big_b).to_u32().unwrap();
            temp /= &big_b;
            idx -= 1;
        }
    }
    
    d[0] = d[k];
    for w in (k + 1)..(l_end + 3) {
        d[w] = d[(w - 1) % k + 1];
    }
    (d, l_end)
}

fn evaluate_rhs(engine: &Engine, ast: &Option<AST>, val: f64, di: u32, d_next: u32, b: u32, rhs_type: u32) -> bool {
    let di_f = di as f64;
    match rhs_type {
        0 => val == di_f,
        1 => val == (b as f64 - 1.0 - di_f),
        2 => val <= di_f,
        3 => val >= di_f,
        4 => val != di_f,
        5 => (val as i64 % 2) == (di as i64 % 2),
        6 => val == (b as f64 / 2.0).floor(),
        7 => val < di_f,
        8 => val > di_f,
        9 => val == ((di as f64 + 1.0) % b as f64),
        10 => val == ((di as f64 + d_next as f64) % b as f64),
        11 => val == ((di as f64 * d_next as f64) % b as f64),
        12 => { 
            if let Some(compiled_ast) = ast {
                let mut scope = Scope::new();
                scope.push("val", val);
                scope.push("di", di_f);
                scope.push("b", b as f64);
                scope.push("d_next", d_next as f64);
                engine.eval_ast_with_scope::<bool>(&mut scope, compiled_ast).unwrap_or(false)
            } else {
                false
            }
        },
        _ => false,
    }
}

pub fn base_solver(n: &BigUint, cfg: &Config, engine: &Engine, lhs_ast: &Option<AST>, rhs1_ast: &Option<AST>, rhs2_ast: &Option<AST>) -> BigUint {
    let (d, loop_end) = get_d_array(n, cfg.b, cfg.k_offset);
    let b = cfg.b;
    let mut total_ways = BigUint::zero();

    let check_conds = |val: f64, di: u32, d_next: u32| -> bool {
        let cond1 = evaluate_rhs(engine, rhs1_ast, val, di, d_next, b, cfg.rhs1);
        if cfg.logic == "NONE" {
            return cond1;
        }
        let cond2 = evaluate_rhs(engine, rhs2_ast, val, di, d_next, b, cfg.rhs2);
        if cfg.logic == "AND" { cond1 && cond2 } else { cond1 || cond2 }
    };

    if cfg.lhs == 14 { 
        let max_sum = (loop_end + 1) * (b as usize - 1);
        let mut dp = vec![BigUint::zero(); max_sum + 1];
        let mut next_dp = vec![BigUint::zero(); max_sum + 1];
        
        for x1 in 0..b {
            for sigma in x1 as usize..=max_sum {
                dp.fill(BigUint::zero());
                dp[x1 as usize] = BigUint::from(1u32);
                let mut possible = true;
                
                for i in 1..=loop_end {
                    let (di, d_next) = (d[i], d[i+1]);
                    next_dp.fill(BigUint::zero());
                    let mut has_state = false;
                    
                    for ci in x1 as usize..=sigma {
                        let ways = &dp[ci];
                        if ways.is_zero() { continue; }
                        for x_next in 0..b {
                            let pi = 2 * (x1 as i64) - (ci as i64);
                            let si = 2 * (x_next as i64) + (ci as i64) - (sigma as i64);
                            let val = (pi.abs() - si.abs()).abs() as f64;
                            if check_conds(val, di, d_next) {
                                let next_c = ci + x_next as usize;
                                if next_c <= sigma {
                                    next_dp[next_c] += ways;
                                    has_state = true;
                                }
                            }
                        }
                    }
                    if !has_state { possible = false; break; }
                    std::mem::swap(&mut dp, &mut next_dp);
                }
                if possible { total_ways += &dp[sigma]; }
            }
        }
    } else if cfg.lhs >= 15 && cfg.lhs <= 18 {
        let num_states = (b * b) as usize;
        let mut v = vec![BigUint::from(1u32); num_states];
        let mut next_v = vec![BigUint::zero(); num_states];
        
        for i in 1..=loop_end {
            let (di, d_next) = (d[i], d[i+1]);
            next_v.fill(BigUint::zero());
            for state in 0..num_states {
                if v[state].is_zero() { continue; }
                let (xi_x, xi_y) = (state % b as usize, state / b as usize);
                
                for next_state in 0..num_states {
                    let (xnext_x, xnext_y) = (next_state % b as usize, next_state / b as usize);
                    let dist = (((xi_x as f64 - xnext_x as f64).powi(2) + (xi_y as f64 - xnext_y as f64).powi(2))).sqrt();
                    let val = match cfg.lhs {
                        16 => dist.floor(),
                        17 => dist.ceil(),
                        18 => dist.round(),
                        _ => dist,
                    };
                    if check_conds(val, di, d_next) { next_v[next_state] += &v[state]; }
                }
            }
            std::mem::swap(&mut v, &mut next_v);
        }
        for ways in v { total_ways += ways; }
    } else if cfg.lhs == 10 || cfg.lhs == 11 {
        let mut v = vec![BigUint::from(1u32); (b * b) as usize];
        let mut next_v = vec![BigUint::zero(); (b * b) as usize];
        
        for i in 1..=loop_end {
            let (di, d_next) = (d[i], d[i+1]);
            next_v.fill(BigUint::zero());
            for a in 0..b {
                for b_val in 0..b {
                    let ways = &v[(a * b + b_val) as usize];
                    if ways.is_zero() { continue; }
                    for c in 0..b {
                        let (xi, x_next, x_nnext, x_prev) = if cfg.lhs == 10 { (b_val, c, 0, a) } else { (a, b_val, c, 0) };
                        let val = if cfg.lhs == 10 {
                            ((x_prev as f64 - xi as f64).abs() - (xi as f64 - x_next as f64).abs()).abs()
                        } else {
                            (xi as f64 - x_next as f64 - x_nnext as f64).abs()
                        };
                        if check_conds(val, di, d_next) { next_v[(b_val * b + c) as usize] += ways; }
                    }
                }
            }
            std::mem::swap(&mut v, &mut next_v);
        }
        for ways in v { total_ways += ways; }
    } else {
        let mut v = vec![BigUint::from(1u32); b as usize];
        let mut next_v = vec![BigUint::zero(); b as usize];
        
        for i in 1..=loop_end {
            let (di, d_next) = (d[i], d[i+1]);
            next_v.fill(BigUint::zero());
            for xi in 0..b {
                if v[xi as usize].is_zero() { continue; }
                for x_next in 0..b {
                    let val = match cfg.lhs {
                        0 => (xi as f64 - x_next as f64).abs(),
                        2 => ((x_next as i32 - xi as i32 + b as i32) % b as i32) as f64,
                        3 => ((xi + x_next) % b) as f64,
                        4 => std::cmp::max(xi, x_next) as f64,
                        5 => std::cmp::min(xi, x_next) as f64,
                        6 => ((xi ^ x_next) % b) as f64,
                        8 => ((xi * x_next) % b) as f64,
                        9 => ((b as f64 - 1.0 - xi as f64) - x_next as f64).abs(),
                        12 => (((xi * xi) % b) as f64 - x_next as f64).abs(),
                        13 => {
                            if let Some(ref ast) = lhs_ast {
                                let mut scope = Scope::new();
                                scope.push("xi", xi as f64);
                                scope.push("x_next", x_next as f64);
                                scope.push("b", b as f64);
                                engine.eval_ast_with_scope::<f64>(&mut scope, ast).unwrap_or(0.0)
                            } else { 
                                0.0 
                            }
                        },
                        _ => (xi as f64 - x_next as f64).abs(),
                    };
                    if check_conds(val, di, d_next) { next_v[x_next as usize] += &v[xi as usize]; }
                }
            }
            std::mem::swap(&mut v, &mut next_v);
        }
        for ways in v { total_ways += ways; }
    }

    total_ways
}

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    let mut r = l;
    let mut g = l;
    let mut b = l;

    if s != 0.0 {
        let hue2rgb = |p: f32, q: f32, mut t: f32| -> f32 {
            if t < 0.0 { t += 1.0; }
            if t > 1.0 { t -= 1.0; }
            if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
            if t < 1.0 / 2.0 { return q; }
            if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
            p
        };

        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;

        r = hue2rgb(p, q, h + 1.0 / 3.0);
        g = hue2rgb(p, q, h);
        b = hue2rgb(p, q, h - 1.0 / 3.0);
    }[
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8
    ]
}

pub fn generate_palette(mod_m: u32) -> Vec<[u8; 3]> {
    let mut p = vec![[30, 30, 45]];
    if mod_m > 1 {
        for i in 1..mod_m {
            let h = (i as f32) / (mod_m as f32);
            p.push(hsl_to_rgb(h, 0.9, 0.55));
        }
    }
    p.push([0, 0, 0]); 
    p
}
