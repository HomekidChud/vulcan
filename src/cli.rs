// Embedded self-test suite: compiles and runs sample programs, verifying output.
use std::process::Command;

// String-builder selftest main — appended after lib/strbuf.vulcan
const STRBUF_TEST_MAIN: &str = r#"
fn main() -> void {
    let s = "one two one two one"
    print(s.replace("one", "1"))
    print(s.replace("zzz", "1"))
    print("no-sep".replace("", "X"))
    let parts = s.split(" ")
    print(parts.join("-"))
    print(["a", "b", "c"].join(", "))
    let b = strbuf_new()
    b.push("hello, ")
    b.push("world")
    b.push_num(7)
    print(b.build())
    print("parts:", b.part_count())
    b.clear()
    b.push("a")
    b.push("b")
    print(b.build_sep("-"))
}
"#;

const STRBUF_TEST_EXPECTED: &str = "1 two 1 two 1\none two one two one\nno-sep\none-two-one-two-one\na, b, c\nhello, world7\nparts: 3\na-b\n";

struct Case {
    name: &'static str,
    src: &'static str,
    expected: &'static str,
}

// Physics engine selftest main — concatenated after lib/physics.vulcan
// (imports are just flattened modules, so this is equivalent to `import physics`).
const PHYSICS_TEST_MAIN: &str = r#"

fn main() -> void {
    let w = world_new(0.0, 900.0)
    w = world_add_bounds(w, 0.5, 0.3)
    w = world_add_circle(w, 40.0, 20.0, 6.0, 1.0, 0.5, 0.2)
    w = world_add_box(w, 120.0, 30.0, 10.0, 8.0, 2.0, 0.2, 0.5)
    w = world_add_spring(w, 4, 5, 50.0, 60.0, 2.0)
    print("bodies:", world_body_count(w), "springs:", world_spring_count(w))
    let f = 0
    while f < 120 {
        w = world_step(w, 1.0 / 60.0)
        f += 1
    }
    let i = 0
    while i < world_body_count(w) {
        print(i, body_x(w, i), body_y(w, i))
        i += 1
    }
}
"#;

const PHYSICS_TEST_EXPECTED: &str = "bodies: 6 springs: 1\n0 128 152\n1 -6 72\n2 262 72\n3 128 -8\n4 59.1166 138.092\n5 108.806 136.102\n";

// Engine selftest main — concatenated after render3d + engine libs
// (the engine's `import render3d` line is stripped; the lib is inlined above it)
const ENGINE_TEST_MAIN: &str = r#"

fn main() -> void {
    let s = scene_new()
    s = scene_flat_terrain(s, 8, 0.0)
    s = scene_spawn(s, "player", ENTITY_KIND_PLAYER, Vec3 { x: 0.0, y: 5.0, z: 0.0 })
    let p = scene_last_id(s)
    s = scene_spawn(s, "bot1", ENTITY_KIND_BOT, Vec3 { x: 4.0, y: 0.5, z: 4.0 })
    let b1 = scene_last_id(s)
    s = scene_spawn(s, "bot2", ENTITY_KIND_BOT, Vec3 { x: -4.0, y: 0.5, z: -4.0 })
    print("spawned:", p, b1, scene_last_id(s), "alive bots:", scene_count_alive(s, ENTITY_KIND_BOT))
    for i in 0 .. 120 {
        s = scene_step(s, 1.0 / 60.0)
    }
    let pi = scene_find(s, p)
    let e = s.entities[pi]
    print("player settled at y:", e.pos.y, "time:", s.time)
    print("terrain at (0,0):", terrain_height(s, 0.0, 0.0), "at (16,16):", terrain_height(s, 16.0, 16.0))
    let bi = scene_find(s, b1)
    s.entities[bi].hp = 0.0
    s = scene_reap(s)
    print("after reap, bots alive:", scene_count_alive(s, ENTITY_KIND_BOT))
    s = engine_tick(s, 0.5, 1.0 / 60.0)
    print("after tick, time:", s.time)
    // ---- v0.2: raycasts, procedural terrain, AI, hitscan, projectiles ----
    let o = Vec3 { x: 0.0, y: 0.0, z: 0.0 }
    let d = Vec3 { x: 0.0, y: 0.0, z: 1.0 }
    print("ray_sphere hit:", ray_sphere(o, d, Vec3 { x: 0.0, y: 0.0, z: 10.0 }, 1.0))
    print("ray_sphere miss:", ray_sphere(o, d, Vec3 { x: 5.0, y: 0.0, z: 10.0 }, 1.0))
    print("ray_aabb hit:", ray_aabb(o, d, Vec3 { x: -1.0, y: -1.0, z: 5.0 }, Vec3 { x: 1.0, y: 1.0, z: 7.0 }))
    print("ray_aabb miss:", ray_aabb(o, d, Vec3 { x: -1.0, y: -1.0, z: 5.0 }, Vec3 { x: 1.0, y: 1.0, z: 6.0 }) == -1.0)
    let t = scene_new()
    t = terrain_sines(t, 16, 4.0)
    print("terrain corners flat:", terrain_height(t, -30.0, -30.0), terrain_height(t, 30.0, 30.0))
    print("terrain center:", terrain_height(t, 0.0, 0.0))
    t = scene_spawn(t, "player", ENTITY_KIND_PLAYER, Vec3 { x: 0.0, y: 1.0, z: 0.0 })
    let tp = scene_last_id(t)
    t = scene_spawn(t, "bot", ENTITY_KIND_BOT, Vec3 { x: 0.0, y: 1.0, z: 8.0 })
    let tb = scene_last_id(t)
    t = bot_think(t, 1.0)
    let tbi = scene_find(t, tb)
    print("bot state:", t.entities[tbi].ai_state, "vel z:", t.entities[tbi].vel.z)
    let tpi = scene_find(t, tp)
    let aim = Vec3 { x: 0.0, y: 0.0, z: 1.0 }
    t = scene_shoot(t, tp, t.entities[tpi].pos, aim, 50.0, 30.0)
    print("bot hp after hit:", t.entities[tbi].hp)
    // settle onto terrain, then fire a projectile from the player at the bot
    t = scene_step(t, 0.1)
    t = scene_step(t, 0.1)
    let muzzle = t.entities[tpi].pos
    let aimdir = t.entities[tbi].pos.sub(muzzle).normalized()
    t = scene_fire_projectile(t, tp, muzzle, aimdir, 10.0, 25.0, 2.0)
    print("entities now:", t.entities.len())
    let steps = 0
    while t.entities[tbi].hp > 45.0 && steps < 40 {
        t = scene_step(t, 0.1)
        steps += 1
    }
    print("bot hp after projectile:", t.entities[tbi].hp, "steps:", steps)
    // ---- v0.3: camera modes, meshes, particles, weapons, save/load ----
    let c1 = cam_first_person(Vec3 { x: 1.0, y: 2.0, z: 3.0 }, 0.5, -0.2)
    print("fp cam:", c1.x, c1.y, c1.z, c1.yaw, c1.pitch)
    let c2 = cam_orbit(Vec3 { x: 0.0, y: 0.0, z: 0.0 }, 0.0, 10.0, 5.0)
    print("orbit cam:", c2.x, c2.y, c2.z)
    let cube = mesh_cube(1.0, 0x8040C0)
    print("cube verts:", cube.verts.len(), "faces:", cube.faces.len())
    let ps = scene_new()
    ps = scene_spawn_particle(ps, Vec3 { x: 1.0, y: 2.0, z: 3.0 }, Vec3 { x: 0.0, y: 1.0, z: 0.0 }, 0.5, 0x00FF00)
    ps = scene_spawn_particle(ps, Vec3 { x: 2.0, y: 2.0, z: 3.0 }, Vec3 { x: 0.0, y: 1.0, z: 0.0 }, 1.5, 0x0000FF)
    print("particles:", ps.particles.len())
    ps = scene_step(ps, 0.1)
    print("after step p0 life:", ps.particles[0].life, "p0 y:", ps.particles[0].pos.y, "p1 life:", ps.particles[1].life)
    ps = scene_step(ps, 0.5)
    print("p0 active:", ps.particles[0].active, "p1 life:", ps.particles[1].life)
    let w = weapon_new("rifle", 25.0, 0.15, true, 60.0, 5, 2.0)
    print("weapon:", w.name, "ammo:", w.ammo, "can fire:", weapon_can_fire(w))
    w = weapon_fire(w)
    print("after fire ammo:", w.ammo, "can fire:", weapon_can_fire(w))
    w = weapon_tick(w, 0.2)
    print("after tick can fire:", weapon_can_fire(w))
    let shots = 0
    while w.ammo > 0 {
        if weapon_can_fire(w) {
            w = weapon_fire(w)
            shots = shots + 1
        }
        w = weapon_tick(w, 0.05)
    }
    print("total shots:", shots, "ammo:", w.ammo)
    w = weapon_reload(w)
    print("reloading:", w.reloading > 0.0)
    w = weapon_tick(w, 2.5)
    print("after reload ammo:", w.ammo, "reloading:", w.reloading)
    let sv = scene_new()
    sv = terrain_sines(sv, 8, 3.0)
    sv = scene_spawn(sv, "player", ENTITY_KIND_PLAYER, Vec3 { x: 1.0, y: 2.0, z: 3.0 })
    sv = scene_spawn(sv, "bot", ENTITY_KIND_BOT, Vec3 { x: -4.0, y: 0.5, z: 6.0 })
    sv.entities[1].hp = 70.0
    scene_save(sv, "selftest_engine_save.txt")
    let ld = scene_load("selftest_engine_save.txt")
    print("loaded time:", ld.time, "gravity:", ld.gravity, "next_id:", ld.next_id)
    print("loaded terrain n:", ld.terrain.len(), "h[0][0]:", ld.terrain[0][0])
    print("loaded ents:", ld.entities.len(), "bot hp:", ld.entities[1].hp, "bot name:", ld.entities[1].name)
    print("loaded bot pos:", ld.entities[1].pos.x, ld.entities[1].pos.y, ld.entities[1].pos.z)
    // ---- v0.4: atmosphere, fog, third-person camera, pickups, compass ----
    let sky0 = sky_colors(0.0)
    let sky30 = sky_colors(30.0)
    let sky90 = sky_colors(90.0)
    print("sky dawn:", sky0[0], sky0[1])
    print("sky noon:", sky30[0], sky30[1])
    print("sky midnight:", sky90[0], sky90[1])
    print("sun light noon:", sun_light(30.0), "midnight:", sun_light(90.0))
    print("fog near:", fog_mix(0xFF0000, 10.0, 0x8ECAE6), "far:", fog_mix(0xFF0000, 200.0, 0x8ECAE6))
    print("sea:", sea_color(0.0, sky0[1]), sea_color(1.0, sky0[1]))
    let c3 = cam_third_person(Vec3 { x: 0.0, y: 0.0, z: 0.0 }, 0.5, 5.0, 0.3)
    print("third cam:", c3.x, c3.y, c3.z, c3.yaw)
    let pk = scene_new()
    pk = scene_spawn_pickup(pk, "medkit", Vec3 { x: 2.0, y: 1.0, z: 2.0 })
    print("pickup:", pk.entities[0].kind, pk.entities[0].name, pk.entities[0].radius)
    draw_sky_gradient(0x0000FF, 0xFF0000, 144)
    hud_compass(0.0, 4, 4, 40)
    print("atmosphere drawn")
    let sc = scene_new()
    sc = scene_spawn(sc, "p", ENTITY_KIND_PLAYER, Vec3 { x: 0.0, y: 0.0, z: 0.0 })
    let pid = scene_last_id(sc)
    sc.camera = cam_first_person(Vec3 { x: 0.0, y: 2.0, z: -10.0 }, 0.0, 0.0)
    sc = player_move_cam(sc, pid, 1.0)
    let pe = sc.entities[scene_find(sc, pid)]
    print("cam move idle:", pe.pos.x, pe.pos.z)
}
"#;

const ENGINE_TEST_EXPECTED: &str = "spawned: 1 2 3 alive bots: 2\nplayer settled at y: 0.5 time: 2\nterrain at (0,0): 0 at (16,16): 0\nafter reap, bots alive: 1\nafter tick, time: 2.1\nray_sphere hit: 9\nray_sphere miss: -1\nray_aabb hit: 5\nray_aabb miss: false\nterrain corners flat: 0 0\nterrain center: 3.76924\nbot state: 3 vel z: -8\nbot hp after hit: 70\nentities now: 4\nbot hp after projectile: 45 steps: 4\nfp cam: 1 3.6 3 0.5 -0.2\norbit cam: 10 5 0\ncube verts: 8 faces: 12\nparticles: 2\nafter step p0 life: 0.4 p0 y: 2.01 p1 life: 1.4\np0 active: false p1 life: 0.9\nweapon: rifle ammo: 5 can fire: true\nafter fire ammo: 4 can fire: false\nafter tick can fire: true\ntotal shots: 4 ammo: 0\nreloading: true\nafter reload ammo: 5 reloading: 0\nloaded time: 0 gravity: -30 next_id: 3\nloaded terrain n: 8 h[0][0]: 0\nloaded ents: 2 bot hp: 70 bot name: bot\nloaded bot pos: -4 0.5 6\nsky dawn: 3483214 10972492\nsky noon: 9292261 13430524\nsky midnight: 461590 1054760\nsun light noon: 1 midnight: 0.22\nfog near: 16711680 far: 10398659\nsea: 3764885 4228009\nthird cam: -2.29006 1.4776 -4.19193 0.5\npickup: 3 medkit 0.4\natmosphere drawn\ncam move idle: 0 0\n";

// 3D renderer selftest main — concatenated after lib/render3d.vulcan
const RENDER3D_TEST_MAIN: &str = r#"

fn main() -> void {
    let cam = Camera { x: 0.0, y: 0.0, z: -10.0, yaw: 0.0, pitch: 0.0 }
    let p = project(cam, Vec3 { x: 0.0, y: 0.0, z: 0.0 })
    print("p:", p.sx, p.sy, p.depth, p.visible)
    let q = project(cam, Vec3 { x: 5.0, y: 5.0, z: 5.0 })
    print("q:", q.sx, q.sy, q.depth, q.visible)
    let b = project(cam, Vec3 { x: 0.0, y: 0.0, z: -20.0 })
    print("b:", b.visible)
    let a = Vec3 { x: 1.0, y: 2.0, z: 3.0 }
    let c = Vec3 { x: -1.0, y: 0.5, z: 2.0 }
    let s = a.add(c)
    print("s:", s.x, s.y, s.z)
    print("dot:", a.dot(c))
    let x = a.cross(c)
    print("cross:", x.x, x.y, x.z)
    print("len:", a.length())
    let n = a.normalized()
    print("n:", n.x, n.y, n.z)
    print("shade:", color_shade(0x8040C0, 0.5))
    print("lerp:", color_lerp(0x000000, 0xFFFFFF, 0.5))
    // depth-buffered triangles: draw far-then-near and near-then-far
    clear(0x000000)
    draw_tri(40.0, 20.0, 50.0, 200.0, 30.0, 50.0, 100.0, 110.0, 50.0, 0x0000FF)
    draw_tri(60.0, 40.0, 10.0, 220.0, 50.0, 10.0, 120.0, 100.0, 10.0, 0xFF0000)
    draw_quad(cam, Vec3 { x: -2.0, y: -2.0, z: 0.0 }, Vec3 { x: 2.0, y: -2.0, z: 0.0 }, Vec3 { x: 2.0, y: 2.0, z: 0.0 }, Vec3 { x: -2.0, y: 2.0, z: 0.0 }, 0x00FF00)
    print("rendered")
}
"#;

const RENDER3D_TEST_EXPECTED: &str = "p: 128 72 10 1\nq: 181.333 18.6667 15 1\nb: 0\ns: 0 2.5 5\ndot: 6\ncross: 2.5 -5 2.5\nlen: 3.74166\nn: 0.267261 0.534522 0.801784\nshade: 4202592\nlerp: 8355711\nrendered\n";

const CASES: &[Case] = &[
    Case {
        name: "basic (0.1 feature set)",
        src: r#"
struct Vec2 { x: f64, y: f64 }

fn len(v: Vec2) -> f64 {
    return sqrt(v.x * v.x + v.y * v.y)
}

fn fib(n: i64) -> i64 {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() -> void {
    print("Vulcan selftest")
    let v = Vec2 { x: 3.0, y: 4.0 }
    print("len(3,4) =", len(v))
    print("fib(20) =", fib(20))
    let arr = [1, 1, 2, 3, 5, 8]
    print("arr sum:")
    let total = 0
    for i in 0 .. arr.len() {
        total = total + arr[i]
    }
    print(total)
    let msg = "hello" + " " + "world"
    print(msg)
}
"#,
        expected: "Vulcan selftest\nlen(3,4) = 5\nfib(20) = 6765\narr sum:\n20\nhello world\n",
    },
    Case {
        name: "elif / compound assign / ++ --",
        src: r#"
fn classify(n: i64) -> string {
    if n < 0 { return "negative" }
    elif n == 0 { return "zero" }
    elif n < 10 { return "small" }
    else { return "big" }
}

fn main() -> void {
    print(classify(-5), classify(0), classify(3), classify(99))
    let x = 10
    x += 5
    x -= 3
    x *= 2
    print(x)
    x++
    x++
    print(x)
    x--
    print(x)
    let f = 2.5
    f /= 0.5
    print(f)
}
"#,
        expected: "negative zero small big\n24\n26\n25\n5\n",
    },
    Case {
        name: "for-in / step ranges / loop-break",
        src: r#"
fn main() -> void {
    let xs = [10, 20, 30]
    let total = 0
    for x in xs { total += x }
    print(total)
    for i in 0 .. 10 by 3 { print(i) }
    for i in 0 ..= 4 by 2 { print(i) }
    for i in 10 .. 0 by -2 { print(i) }
    let n = 0
    loop {
        n++
        if n >= 3 { break }
    }
    print("looped", n)
}
"#,
        expected: "60\n0\n3\n6\n9\n0\n2\n4\n10\n8\n6\n4\n2\nlooped 3\n",
    },
    Case {
        name: "match / enums",
        src: r#"
enum Color { Red, Green, Blue }

fn name(c: Color) -> string {
    match c {
        when Color.Red { return "red" }
        when Color.Green { return "green" }
        else { return "blue" }
    }
}

fn main() -> void {
    print(name(Color.Red), name(Color.Green), name(Color.Blue))
    let c = Color.Blue
    match c {
        when Color.Red { print("nope") }
        when Color.Blue, Color.Green { print("cool color") }
    }
    let m = "b"
    match m {
        when "a" { print("letter a") }
        when "b" { print("letter b") }
        else { print("other") }
    }
}
"#,
        expected: "red green blue\ncool color\nletter b\n",
    },
    Case {
        name: "f-strings / consts / string methods",
        src: r#"
const LIMIT: i64 = 42
const APP: string = "vulcan"

fn main() -> void {
    let name = "ada"
    let n = 7
    print(f"{APP} says: {name} scored {n * 6} points")
    print(f"limit is {LIMIT}")
    print("SHOUT".lower(), "quiet".upper())
    print("  padded  ".trim())
    print(f"nested: {[1,2][1]}")
}
"#,
        expected: "vulcan says: ada scored 42 points\nlimit is 42\nshout QUIET\npadded\nnested: 2\n",
    },
    Case {
        name: "defer / assert",
        src: r#"
fn work() -> i64 {
    defer { print("cleanup") }
    print("working")
    return 5
}

fn main() -> void {
    print(work())
    assert(1 + 1 == 2)
    let items = [1, 2, 3]
    assert(items.len() == 3, "array size")
    print("after asserts")
}
"#,
        expected: "working\ncleanup\n5\nafter asserts\n",
    },
    Case {
        name: "bitwise / hex / binary / chars / casts",
        src: r#"
fn main() -> void {
    let flags = 0b1010
    print(flags)
    print(0xFF)
    print(flags & 0b0110)
    print(flags | 1)
    print(flags ^ 0b1111)
    print(flags << 2)
    print(flags >> 1)
    print(~0)
    print('A')
    let x = 3
    let y = x as f64 / 2.0
    print(y)
    print('z' - 'a')
}
"#,
        expected: "10\n255\n2\n11\n5\n40\n5\n-1\n65\n1.5\n25\n",
    },
    Case {
        name: "array methods / min max / pop",
        src: r#"
fn main() -> void {
    let xs = [5, 3, 8]
    print(xs.pop())
    print(xs.len())
    print(xs.last())
    print(xs.contains(3), xs.contains(99))
    print(min(4, 9), max(4, 9))
    print(min(2.5, 1.5), max(2.5, 1.5))
    print(abs(-7), abs(-2.5))
    print(pow(2.0, 10.0))
    print(floor(2.7), ceil(2.1))
}
"#,
        expected: "8\n2\n3\ntrue false\n4 9\n1.5 2.5\n7 2.5\n1024\n2 3\n",
    },
    Case {
        name: "structs with arrays + compound field update",
        src: r#"
struct Player { name: string, hp: i64 }

fn main() -> void {
    let party = [Player { name: "ada", hp: 100 }, Player { name: "bob", hp: 80 }]
    party[0].hp -= 30
    party[1].hp += 10
    print(party[0].name, party[0].hp)
    print(party[1].name, party[1].hp)
    let v = [1, 2, 3]
    v[1] += 10
    print(v[1])
}
"#,
        expected: "ada 70\nbob 90\n12\n",
    },
    Case {
        name: "test runner (vulcan test)",
        src: r#"
fn add(a: i64, b: i64) -> i64 { return a + b }

fn test_add() -> void {
    assert(add(2, 3) == 5)
}

fn test_negatives() -> void {
    assert(add(-2, 3) == 1)
}

fn main() -> void {
    print("main is not run in test mode")
}
"#,
        expected: "running 2 test(s)\n  test_add ...\n  test_negatives ...\nall tests passed\n",
    },
    Case {
        name: "game stdlib (rand_int, ms_now, bare return)",
        src: r#"
fn main() -> void {
    // rand_int stays in bounds
    let ok = true
    for i in 0 .. 50 {
        let r = rand_int(3, 7)
        if r < 3 || r > 7 { ok = false }
    }
    print("rand bounds:", ok)
    // ms_now is a monotonic millisecond clock
    let a = ms_now()
    let b = ms_now()
    print("clock sane:", b >= a)
    // bare `return` (no value, no semicolon)
    print("before")
    if a >= 0 {
        return
    }
    print("unreachable")
}
"#,
        expected: "rand bounds: true\nclock sane: true\nbefore\n",
    },
    Case {
        name: "snake-style loop + string repeat",
        src: r#"
fn main() -> void {
    let xs: [string] = []
    for c in ["a", "b", "c"] {
        xs.push(c)
    }
    print(xs)
    print(".".repeat(3))
}
"#,
        expected: "[a, b, c]\n...\n",
    },
    Case {
        name: "impl methods (all backends)",
        src: r#"
struct Vec2 { x: f64, y: f64 }

impl Vec2 {
    fn move(self, dx: f64, dy: f64) -> Vec2 {
        self.x = self.x + dx
        self.y = self.y + dy
        return self
    }
    fn len2(self) -> f64 {
        return self.x * self.x + self.y * self.y
    }
    fn scale(self, k: f64) -> Vec2 {
        self.x = self.x * k
        self.y = self.y * k
        return self
    }
}

fn main() -> void {
    let p = Vec2 { x: 1.0, y: 2.0 }
    p.move(3.0, 4.0)
    print("p =", p.x, p.y)
    print("len2 =", p.len2())
    p.scale(2.0)
    print("scaled =", p.x, p.y)
    let pts: [Vec2] = [Vec2 { x: 1.0, y: 1.0 }, Vec2 { x: 2.0, y: 3.0 }]
    print("pts[1].y =", pts[1].y)
}
"#,
        expected: "p = 4 6\nlen2 = 52\nscaled = 8 12\npts[1].y = 3\n",
    },
    Case {
        name: "small numeric types (all backends)",
        src: r#"
struct Pixel { r: u8, g: u8, b: u8, a: u8 }
fn half(n: i32) -> i32 { return n / 2 }

fn main() -> void {
    let a: i32 = 5
    let b: i32 = 3
    print("i32:", a + b, a * b, a - b, a / b, a % b)
    let big: i64 = 300
    print("u8 wrap:", big as u8)
    print("casts:", 1000 as u16, 2.9 as i64, (-7) as u8, 70000 as u16)
    let f: f32 = 0.5
    let g: f32 = 0.25
    print("f32:", f + g, f * 4.0, f / g)
    let h: f32 = 16777217
    print("f32 round:", h)
    let arr: [i32] = [1, 2, 3]
    arr.push(300)
    print("arr:", arr.len(), arr[0], arr[3], arr[1] + arr[2])
    let fa: [f32] = [0.5, 0.25]
    print("farr:", fa[0] + fa[1])
    let p = Pixel { r: 255, g: 128, b: 0, a: 300 }
    print("pixel:", p.r, p.g, p.a)
    p.r = p.r + 1
    print("pixel mut:", p.r)
    let x: u8 = 250
    x += 10
    print("u8 +=:", x)
    x++
    print("u8 ++:", x)
    let xs: [i8] = [120, 121]
    xs[0] += 10
    print("i8 arr:", xs[0])
    let neg: i8 = -128
    print("i8:", neg, neg - 1)
    let u: u32 = 4000000000
    print("u32:", u)
    print("fn:", half(9), half(-9))
}
"#,
        expected: "i32: 8 15 2 1 2\nu8 wrap: 44\ncasts: 1000 2 249 4464\nf32: 0.75 2 2\nf32 round: 1.67772e+07\narr: 4 1 300 5\nfarr: 0.75\npixel: 255 128 44\npixel mut: 0\nu8 +=: 4\nu8 ++: 5\ni8 arr: -126\ni8: -128 -129\nu32: 4000000000\nfn: 4 -4\n",
    },
    Case {
        name: "const folding (all backends)",
        src: r#"
const N: i64 = 16 * 4
const HALF: f64 = 1.0 / 2.0
const NEG: i64 = -(3 + 4)
const GREET: string = "hi" + " " + "there"
const FLAG: bool = 1 < 2 && 3 > 2

fn main() -> void {
    print(N, HALF, NEG, GREET, FLAG)
    let xs: [i64] = []
    for i in 0 .. N / 16 {
        xs.push(i)
    }
    print("len:", xs.len())
}
"#,
        expected: "64 0.5 -7 hi there true\nlen: 4\n",
    },
    Case {
        name: "match guards (all backends)",
        src: r#"
fn classify(x: i64) -> string {
    match x {
        when 5, 10 if x > 7 { return "big-mid" }
        when 5, 10 { return "small-mid" }
        when 1 if false { return "never" }
        else { return "other" }
    }
}

fn main() -> void {
    let xs = [1, 5, 10, 15, 20]
    for x in xs {
        print(x, classify(x))
    }
    let s = "hit"
    match s {
        when "hit" if s.len() == 3 { print("str guard ok") }
        else { print("str guard fail") }
    }
}
"#,
        expected: "1 other\n5 small-mid\n10 big-mid\n15 other\n20 other\nstr guard ok\n",
    },
    Case {
        name: "struct value semantics (all backends)",
        src: r#"
struct Inner { v: i64 }
struct Box { inner: Inner, xs: [i64] }

fn mutate(p: Inner) -> void {
    p.v = 99
}

fn main() -> void {
    let a = Inner { v: 1 }
    let b = a
    b.v = 42
    print("copy:", a.v, b.v)
    mutate(a)
    print("arg:", a.v)
    let box1 = Box { inner: Inner { v: 7 }, xs: [1, 2, 3] }
    let box2 = box1
    box2.inner.v = 10
    box2.xs[0] = 100
    print("deep:", box1.inner.v, box1.xs[0], box2.inner.v, box2.xs[0])
    let arr: [Inner] = [Inner { v: 1 }]
    let el = arr[0]
    el.v = 55
    print("elem:", arr[0].v, el.v)
    for it in arr {
        it.v = 77
    }
    print("forin:", arr[0].v)
    let lit = [a, a]
    a.v = 5
    print("arrlit:", lit[0].v, lit[1].v)
    let b2 = Box { inner: a, xs: [7, 8] }
    a.v = 9
    print("structlit:", b2.inner.v)
}
"#,
        expected: "copy: 1 42\narg: 1\ndeep: 7 100 10 100\nelem: 1 55\nforin: 1\narrlit: 1 1\nstructlit: 5\n",
    },
    Case {
        name: "method calls on expressions (all backends)",
        src: r#"
struct Vec2 { x: f64, y: f64 }
impl Vec2 {
    fn add(self, o: Vec2) -> Vec2 {
        self.x = self.x + o.x
        self.y = self.y + o.y
        return self
    }
}
fn origin() -> Vec2 { return Vec2 { x: 0.0, y: 0.0 } }

fn main() -> void {
    let a = Vec2 { x: 1.0, y: 2.0 }
    let b = Vec2 { x: 10.0, y: 20.0 }
    let c = a.add(b).add(b)
    print("chained:", c.x, c.y)
    let d = origin().add(a)
    print("on call:", d.x, d.y)
    let pts: [Vec2] = [a, b]
    let e = pts[0].add(b)
    print("on elem:", e.x, e.y)
}
"#,
        expected: "chained: 21 42\non call: 11 22\non elem: 21 42\n",
    },
    Case {
        name: "forward references between functions (all backends)",
        src: r#"
fn early(x: i64) -> i64 { return later(x) + 1 }
fn fwd2(x: i64) -> i64 { return early(x) - 1 }
fn later(x: i64) -> i64 { return x * 2 }

fn main() -> void {
    print(early(5))
    print(fwd2(10))
}
"#,
        expected: "11\n20\n",
    },
    Case {
        name: "file I/O, args, string/array stdlib",
        src: r#"
fn main() -> void {
    // file I/O
    file_write("selftest_io.txt", "alpha,beta,gamma")
    print(file_exists("selftest_io.txt"))
    let raw = file_read("selftest_io.txt")
    print(raw)
    // string stdlib
    let parts = raw.split(",")
    print(parts.len())
    print(parts[1])
    print(raw.starts_with("alpha"))
    print(raw.ends_with("gamma"))
    print(raw.substring(6, 10))
    print(raw.find("beta"))
    print("SHOUT".lower())
    print("  pad  ".trim())
    print(to_int("42"), to_int("bad"))
    print(to_float("2.5"))
    // array stdlib
    let xs = [1, 2, 3, 4]
    xs.remove(0)
    xs.insert(1, 99)
    print(xs)
    xs.push(5)
    print(xs.last())
    print(xs.contains(99))
}
"#,
        expected: "true\nalpha,beta,gamma\n3\nbeta\ntrue\ntrue\nbeta\n6\nshout\npad\n42 0\n2.5\n[2, 99, 3, 4]\n5\ntrue\n",
    },
    Case {
        name: "graphics builtins (all backends)",
        src: r#"
fn main() -> void {
    print("canvas:", canvas_width(), "x", canvas_height())
    clear(0x102030)
    draw_rect(10.5, 20.5, 50.0, 30.0, 0xFF5500)
    draw_circle(200.0, 70.0, 25.0, 0x33CCFF)
    draw_line(0.0, 0.0, 255.0, 143.0, 0xFFFFFF)
    draw_text(4.0, 4.0, "VULCAN", 0xFFFFFF)
    draw_text(4.0, 130.0, "count: 42", 0x00FF00)
    print("drawn")
}
"#,
        expected: "canvas: 256 x 144\ndrawn\n",
    },
    Case {
        name: "ffi extern (all backends)",
        src: r#"
extern "c" {
    fn sqrt(x: f64) -> f64
    fn sin(x: f64) -> f64
    fn floor(x: f64) -> f64
    fn ceil(x: f64) -> f64
}

fn main() -> void {
    print(sqrt(16.0))
    print(sin(0.0))
    print(floor(3.9))
    print(ceil(3.1))
}
"#,
        expected: "4\n0\n3\n4\n",
    },
    Case {
        name: "math builtins: atan2 / tan / exp / log (all backends)",
        src: r#"
fn main() -> void {
    print("atan2:", atan2(0.0, 1.0), atan2(1.0, 1.0), atan2(0.0, -1.0))
    print("tan:", tan(0.0))
    print("exp:", exp(0.0), exp(1.0))
    print("log:", log(1.0))
}
"#,
        expected: "atan2: 0 0.785398 3.14159\ntan: 0\nexp: 1 2.71828\nlog: 0\n",
    },
    Case {
        name: "nested arrays (all backends)",
        src: r#"
struct Level { name: string, tiles: [[i64]] }

fn make_grid(rows: i64, cols: i64) -> [[i64]] {
    let g: [[i64]] = []
    for r in 0 .. rows {
        let row: [i64] = []
        for c in 0 .. cols {
            row.push(r * 10 + c)
        }
        g.push(row)
    }
    return g
}

fn main() -> void {
    let g = make_grid(3, 4)
    print("g[1][2] =", g[1][2], "len:", g.len(), "row1:", g[1].len())
    g[0][0] = 99
    g[1][1] += 5
    print("mutated:", g[0][0], g[1][1])
    let lvl = Level { name: "world", tiles: [[1, 2], [3, 4]] }
    lvl.tiles[1][0] = 30
    print("level:", lvl.name, lvl.tiles[1][0], lvl.tiles[0][1])
    let cube: [[[i64]]] = [[[1, 2]], [[3, 4], [5, 6]]]
    print("cube:", cube[1][0][1], cube[1][1][0])
    let cells: [[Cell]] = [[Cell { x: 1, y: 2 }, Cell { x: 3, y: 4 }], [Cell { x: 5, y: 6 }]]
    print("cells:", cells[0][1].x, cells[1][0].y)
    let popped: [i64] = g.pop()
    print("popped len:", popped.len(), "grid len:", g.len())
}
struct Cell { x: i64, y: i64 }
"#,
        expected: "g[1][2] = 12 len: 3 row1: 4\nmutated: 99 16\nlevel: world 30 2\ncube: 4 5\ncells: 3 6\npopped len: 4 grid len: 2\n",
    },
    Case {
        name: "payload enums / Option / Result / ?",
        src: r#"
enum Shape { Circle(f64), Rect(f64, f64), Point }

struct P { x: i64, y: i64 }
enum Item { Key(P), Name(string) }

fn area(s: Shape) -> f64 {
    match s {
        when Shape.Circle(r) if r > 1.0 { return 3.14159 * r * r }
        when Shape.Circle(r) { return 0.0 }
        when Shape.Rect(w, h) { return w * h }
        when Shape.Point { return 0.0 }
    }
    return -1.0
}

fn find_first_ge(xs: [i64], threshold: i64) -> Option<i64> {
    for x in xs {
        if x >= threshold {
            return Option.Some(x)
        }
    }
    return Option.None
}

fn div_checked(a: i64, b: i64) -> Result<i64, string> {
    if b == 0 {
        return Result.Err("division by zero")
    }
    return Result.Ok(a / b)
}

fn use_q(a: i64, b: i64) -> Result<i64, string> {
    let d = div_checked(a, b)
    let q = d?
    return Result.Ok(q * 2)
}

fn main() -> void {
    print("circle:", area(Shape.Circle(2.0)))
    print("small circle:", area(Shape.Circle(0.5)))
    print("rect:", area(Shape.Rect(3.0, 4.0)))
    print("point:", area(Shape.Point))
    match find_first_ge([1, 5, 9], 6) {
        when Option.Some(v) { print("found:", v) }
        when Option.None { print("none") }
    }
    match find_first_ge([1, 2], 10) {
        when Option.Some(v) { print("found:", v) }
        when Option.None { print("none") }
    }
    match div_checked(10, 2) {
        when Result.Ok(v) { print("ok:", v) }
        when Result.Err(m) { print("err:", m) }
    }
    match div_checked(1, 0) {
        when Result.Ok(v) { print("ok:", v) }
        when Result.Err(m) { print("err:", m) }
    }
    match use_q(10, 5) {
        when Result.Ok(v) { print("q ok:", v) }
        when Result.Err(m) { print("q err:", m) }
    }
    match use_q(1, 0) {
        when Result.Ok(v) { print("q ok:", v) }
        when Result.Err(m) { print("q err:", m) }
    }
    let a = Item.Key(P { x: 1, y: 2 })
    let b = a
    match b {
        when Item.Key(p) {
            p.x = 99
            print("b key:", p.x, p.y)
        }
        else { print("b other") }
    }
    match a {
        when Item.Key(p) { print("a key:", p.x, p.y) }
        else { print("a other") }
    }
    match Item.Name("hi") {
        when Item.Name(n) { print("name:", n) }
        else { print("other") }
    }
}
"#,
        expected: "circle: 12.5664\nsmall circle: 0\nrect: 12\npoint: 0\nfound: 9\nnone\nok: 5\nerr: division by zero\nq ok: 4\nq err: division by zero\nb key: 99 2\na key: 1 2\nname: hi\n",
    },
    Case {
        name: "clamp / lerp / sign (all backends)",
        src: r#"
fn main() -> void {
    print("clamp:", clamp(5, 0, 3), clamp(-1, 0, 3), clamp(2.5, 0.0, 2.0))
    print("lerp:", lerp(0.0, 10.0, 0.25), lerp(10.0, 20.0, 0.5))
    print("sign:", sign(-3), sign(0), sign(7), sign(-2.5), sign(0.0), sign(3.5))
}
"#,
        expected: "clamp: 3 0 2\nlerp: 2.5 15\nsign: -1 0 1 -1 0 1\n",
    },
];

// ---- `vulcan doc` — print the public API of a file (or every lib/*.vulcan) ----

fn doc_ty(t: &crate::parser::Ty) -> String {
    use crate::parser::Ty::*;
    match t {
        I64 => "i64".into(), F64 => "f64".into(), Bool => "bool".into(),
        Str => "string".into(), Void => "void".into(),
        I8 => "i8".into(), I16 => "i16".into(), I32 => "i32".into(),
        U8 => "u8".into(), U16 => "u16".into(), U32 => "u32".into(),
        F32 => "f32".into(),
        Array(inner) => format!("[{}]", doc_ty(inner)),
        Option(p) => format!("Option<{}>", doc_ty(p)),
        Result(o, e) => format!("Result<{}, {}>", doc_ty(o), doc_ty(e)),
        Struct(n) => n.clone(),
    }
}

pub fn doc(path: &str) {
    use crate::parser::{Item, ConstVal};
    let p = std::path::Path::new(path);
    let files: Vec<(String, String)> = if p.is_dir() {
        let mut v = vec![];
        if let Ok(rd) = std::fs::read_dir(p) {
            let mut names: Vec<_> = rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|e| e.extension().map(|x| x == "vulcan").unwrap_or(false))
                .collect();
            names.sort();
            for n in names {
                if let Ok(src) = std::fs::read_to_string(&n) {
                    v.push((n.file_name().unwrap().to_string_lossy().into_owned(), src));
                }
            }
        }
        v
    } else {
        match std::fs::read_to_string(p) {
            Ok(src) => vec![(p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(), src)],
            Err(e) => { eprintln!("vulcan: cannot read {path}: {e}"); std::process::exit(1); }
        }
    };
    if files.is_empty() {
        eprintln!("vulcan: no .vulcan files found in {path}");
        std::process::exit(1);
    }
    for (name, src) in files {
        println!("=== {name} ===");
        let program = crate::parser::parse_program(&crate::lexer::lex(&src));
        for item in program {
            match item {
                Item::Const(c) => {
                    let val = match c.value {
                        ConstVal::Int(v) => v.to_string(),
                        ConstVal::Float(v) => format!("{v}"),
                        ConstVal::Str(v) => format!("{v:?}"),
                        ConstVal::Bool(v) => v.to_string(),
                    };
                    println!("const {} = {}", c.name, val);
                }
                Item::Struct(s) => {
                    println!("struct {} {{", s.name);
                    for (f, t) in s.fields {
                        println!("    {}: {},", f, doc_ty(&t));
                    }
                    println!("}}");
                }
                Item::Fn(f) => {
                    let params = f.params.iter().map(|(n, t)| format!("{}: {}", n, doc_ty(t)))
                        .collect::<Vec<_>>().join(", ");
                    println!("fn {}({}) -> {}", f.name, params, doc_ty(&f.ret));
                }
                Item::Impl(i) => {
                    for m in &i.fns {
                        let params = m.params.iter().map(|(n, t)| format!("{}: {}", n, doc_ty(t)))
                            .collect::<Vec<_>>().join(", ");
                        println!("impl {}: fn {}({}) -> {}", i.struct_name, m.name, params, doc_ty(&m.ret));
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn selftest() {
    let dir = std::env::temp_dir().join("vulcan_selftest");
    let _ = std::fs::create_dir_all(&dir);
    let mut failures = 0;
    // the physics engine case: lib source + test main, verified like any other case
    let physics_src = include_str!("../lib/physics.vulcan").to_string() + PHYSICS_TEST_MAIN;
    failures += run_case("physics engine (all backends)", &physics_src, PHYSICS_TEST_EXPECTED, &dir);
    // the 3D renderer case: lib source + test main
    let render3d_src = include_str!("../lib/render3d.vulcan").to_string() + RENDER3D_TEST_MAIN;
    failures += run_case("3d renderer (all backends)", &render3d_src, RENDER3D_TEST_EXPECTED, &dir);
    // the engine case: render3d + engine libs (import stripped) + test main
    let engine_src = include_str!("../lib/render3d.vulcan").to_string()
        + &include_str!("../lib/engine.vulcan").replace("import render3d\n", "");
    let engine_src = engine_src + ENGINE_TEST_MAIN;
    failures += run_case("engine (all backends)", &engine_src, ENGINE_TEST_EXPECTED, &dir);
    // the string-builder case: lib source + test main
    let strbuf_src = include_str!("../lib/strbuf.vulcan").to_string() + STRBUF_TEST_MAIN;
    failures += run_case("string replace/join + strbuf (all backends)", &strbuf_src, STRBUF_TEST_EXPECTED, &dir);
    for case in CASES {
        failures += run_case(case.name, case.src, case.expected, &dir);
    }
    if failures > 0 {
        eprintln!("{failures} selftest case(s) failed");
        std::process::exit(1);
    }
    println!("vulcan selftest: {} cases passed", CASES.len() + 4);
}

fn run_case(name: &str, src: &str, expected: &str, dir: &std::path::Path) -> usize {
    let mut failures = 0;
    let tag = name.replace(['(', ')', ' ', '/'], "_");
    let src_path = dir.join(format!("{tag}.vulcan"));
    std::fs::write(&src_path, src).unwrap();
    let c_path = dir.join(format!("{tag}.v.c"));
    let out_path = dir.join(format!("{tag}.vbin"));
    let tokens = crate::lexer::lex(src);
    let program = crate::parser::parse_program(&tokens);
    let c_src = if name.starts_with("test runner") {
        let tests: Vec<String> = program.iter().filter_map(|i| match i {
            crate::parser::Item::Fn(f) if f.name.starts_with("test_") => Some(f.name.clone()),
            _ => None,
        }).collect();
        crate::codegen::generate_test(&program, &tests)
    } else {
        crate::codegen::generate(&program)
    };
    std::fs::write(&c_path, c_src).unwrap();
    let cc = crate::pick_cc();
    let status = match Command::new(&cc)
        .args(["-O3", c_path.to_str().unwrap(), "-lm", "-o", out_path.to_str().unwrap()])
        .output()
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[FAIL] {}: cannot run C compiler `{cc}`: {e}\n  (install gcc or clang, or set VULCAN_CC)", name);
            return 1;
        }
    };
    if !status.status.success() {
        eprintln!("[FAIL] {}: compile failed:\n{}", name, String::from_utf8_lossy(&status.stderr));
        return 1;
    }
    let run = match Command::new(&out_path).output() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[FAIL] {}: cannot run {}: {e}", name, out_path.display());
            return 1;
        }
    };
    let stdout = String::from_utf8_lossy(&run.stdout);
    if stdout != expected {
        eprintln!("[FAIL] {}: output mismatch\n--- got ---\n{stdout}--- expected ---\n{}", name, expected);
        failures += 1;
    } else {
        println!("[ok] {}", name);
    }
    // cross-backend check: the JS and Python backends must produce identical output
    if name.starts_with("test runner") { return failures; }
    for backend in ["js", "py"] {
        let (runner, src) = if backend == "js" {
            ("node", crate::codegen_js::generate_js(&program))
        } else {
            ("python3", crate::codegen_py::generate_py(&program))
        };
        let script_path = dir.join(format!("{tag}.{backend}"));
        std::fs::write(&script_path, &src).unwrap();
        match Command::new(runner).arg(&script_path).output() {
            Ok(r) => {
                let stdout = String::from_utf8_lossy(&r.stdout);
                if stdout != expected {
                    eprintln!("[FAIL] {}: {backend} backend output mismatch\n--- got ---\n{stdout}--- expected ---\n{}", name, expected);
                    failures += 1;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("[skip] {}: `{runner}` not found — {backend} backend not verified", name);
            }
            Err(e) => { eprintln!("[FAIL] {}: {backend} runner error: {e}", name); failures += 1; }
        }
    }
    failures
}
