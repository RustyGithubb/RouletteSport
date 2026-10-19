//! Filtre « bitmap » : vrai post-traitement de l'image finale (shader OpenGL).
//! L'écran déjà dessiné est copié dans une texture puis redessiné en gros pixels,
//! avec une palette réduite et un tramage ordonné (Bayer 4×4), façon écran 1 bit.

use eframe::egui::{self, Color32, LayerId, PaintCallback, Rect};
use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum BitmapMode {
    /// 1 bit (ou N niveaux) entre le fond et le texte de la palette.
    Palette,
    /// Niveaux de gris.
    Gray,
    /// Couleurs réduites (N niveaux par canal).
    Color,
}

impl BitmapMode {
    pub const ALL: [BitmapMode; 3] = [BitmapMode::Palette, BitmapMode::Gray, BitmapMode::Color];
    pub fn label(self) -> &'static str {
        match self {
            BitmapMode::Palette => "Palette (fond / texte)",
            BitmapMode::Gray => "Niveaux de gris",
            BitmapMode::Color => "Couleurs réduites",
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct BitmapFilter {
    pub enabled: bool,
    /// Taille d'un pixel du filtre, en points (1..10).
    pub pixel: f32,
    pub mode: BitmapMode,
    /// Nombre de niveaux (2 = 1 bit).
    pub levels: u32,
    /// Force du tramage (0..1).
    pub dither: f32,
    /// Mélange avec l'image d'origine (0..1).
    pub intensity: f32,
}

impl Default for BitmapFilter {
    fn default() -> Self {
        Self { enabled: false, pixel: 1.0, mode: BitmapMode::Color, levels: 3, dither: 1.0, intensity: 0.35 }
    }
}

struct GlRes {
    program: glow::Program,
    tex: glow::Texture,
    vbo: glow::Buffer,
    vao: Option<glow::VertexArray>,
}

/// Ressources OpenGL du filtre, créées au premier dessin.
#[derive(Clone, Default)]
pub struct BitmapGl(Arc<Mutex<Option<Result<GlRes, String>>>>);

const VS: &str = r#"
IN vec2 a_pos;
void main() { gl_Position = vec4(a_pos, 0.0, 1.0); }
"#;

const FS: &str = r#"
uniform sampler2D u_tex;
uniform vec2 u_size;
uniform float u_pixel;
uniform float u_levels;
uniform float u_dither;
uniform float u_mix;
uniform float u_mode;
uniform vec3 u_ink;
uniform vec3 u_paper;
OUT_DECL

float bayer2(vec2 a) { a = floor(a); return fract(dot(a, vec2(0.5, a.y * 0.75))); }
float bayer4(vec2 a) { return bayer2(0.5 * a) * 0.25 + bayer2(a); }

void main() {
    vec2 frag = gl_FragCoord.xy;
    vec2 cell = floor(frag / u_pixel);
    vec2 center = min((cell + 0.5) * u_pixel, u_size - 0.5);
    vec3 orig = TEX(u_tex, frag / u_size).rgb;
    vec3 c = TEX(u_tex, center / u_size).rgb;
    float n = max(u_levels - 1.0, 1.0);
    float t = (bayer4(cell) - 0.5) * u_dither;
    vec3 outc;
    if (u_mode < 1.5) {
        float l = dot(c, vec3(0.299, 0.587, 0.114));
        // Distance entre fond et texte de la palette, pour un 1 bit fidèle au thème.
        float lp = dot(u_paper, vec3(0.299, 0.587, 0.114));
        float li = dot(u_ink, vec3(0.299, 0.587, 0.114));
        if (u_mode < 0.5 && abs(li - lp) > 0.02) { l = clamp((l - lp) / (li - lp), 0.0, 1.0); }
        float q = clamp(floor(l * n + 0.5 + t), 0.0, n) / n;
        outc = u_mode < 0.5 ? mix(u_paper, u_ink, q) : vec3(q);
    } else {
        outc = clamp(floor(c * n + 0.5 + t), 0.0, n) / n;
    }
    FRAG = vec4(mix(orig, outc, u_mix), 1.0);
}
"#;

unsafe fn compile(gl: &glow::Context) -> Result<GlRes, String> {
    let sv = egui_glow::ShaderVersion::get(gl);
    let es = matches!(sv, egui_glow::ShaderVersion::Es100 | egui_glow::ShaderVersion::Es300);
    let head = format!("{}{}", sv.version_declaration(), if es { "precision mediump float;\n" } else { "" });
    let (vs, fs) = if sv.is_new_shader_interface() {
        (
            format!("{head}#define IN in\n{VS}"),
            format!("{head}#define TEX texture\n#define FRAG f_color\n{}", FS.replace("OUT_DECL", "out vec4 f_color;")),
        )
    } else {
        (
            format!("{head}#define IN attribute\n{VS}"),
            format!("{head}#define TEX texture2D\n#define FRAG gl_FragColor\n{}", FS.replace("OUT_DECL", "")),
        )
    };
    let program = gl.create_program()?;
    let mut shaders = vec![];
    for (kind, src) in [(glow::VERTEX_SHADER, vs), (glow::FRAGMENT_SHADER, fs)] {
        let s = gl.create_shader(kind)?;
        gl.shader_source(s, &src);
        gl.compile_shader(s);
        if !gl.get_shader_compile_status(s) {
            return Err(gl.get_shader_info_log(s));
        }
        gl.attach_shader(program, s);
        shaders.push(s);
    }
    gl.bind_attrib_location(program, 0, "a_pos");
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        return Err(gl.get_program_info_log(program));
    }
    for s in shaders {
        gl.detach_shader(program, s);
        gl.delete_shader(s);
    }
    let tex = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    for (k, v) in [
        (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
        (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
        (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
    ] {
        gl.tex_parameter_i32(glow::TEXTURE_2D, k, v as i32);
    }
    // Triangle unique qui couvre tout l'écran.
    let verts: [f32; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];
    let bytes: Vec<u8> = verts.iter().flat_map(|v| v.to_ne_bytes()).collect();
    let vbo = gl.create_buffer()?;
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
    let vao = gl.create_vertex_array().ok();
    Ok(GlRes { program, tex, vbo, vao })
}

unsafe fn draw(gl: &glow::Context, r: &GlRes, f: &BitmapFilter, w: i32, h: i32, ppp: f32, ink: Color32, paper: Color32) {
    // 1. copie de l'image déjà dessinée
    gl.active_texture(glow::TEXTURE0);
    gl.bind_texture(glow::TEXTURE_2D, Some(r.tex));
    gl.copy_tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA, 0, 0, w, h, 0);
    // 2. redessin plein écran avec le shader
    gl.viewport(0, 0, w, h);
    gl.disable(glow::SCISSOR_TEST);
    gl.disable(glow::BLEND);
    gl.use_program(Some(r.program));
    let u = |n: &str| gl.get_uniform_location(r.program, n);
    let rgb = |c: Color32| [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0];
    gl.uniform_1_i32(u("u_tex").as_ref(), 0);
    gl.uniform_2_f32(u("u_size").as_ref(), w as f32, h as f32);
    gl.uniform_1_f32(u("u_pixel").as_ref(), (f.pixel * ppp).round().max(1.0));
    gl.uniform_1_f32(u("u_levels").as_ref(), f.levels.clamp(2, 32) as f32);
    gl.uniform_1_f32(u("u_dither").as_ref(), f.dither.clamp(0.0, 1.0));
    gl.uniform_1_f32(u("u_mix").as_ref(), f.intensity.clamp(0.0, 1.0));
    let mode = match f.mode {
        BitmapMode::Palette => 0.0,
        BitmapMode::Gray => 1.0,
        BitmapMode::Color => 2.0,
    };
    gl.uniform_1_f32(u("u_mode").as_ref(), mode);
    let (i, p) = (rgb(ink), rgb(paper));
    gl.uniform_3_f32(u("u_ink").as_ref(), i[0], i[1], i[2]);
    gl.uniform_3_f32(u("u_paper").as_ref(), p[0], p[1], p[2]);
    if let Some(vao) = r.vao {
        gl.bind_vertex_array(Some(vao));
    }
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(r.vbo));
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
    gl.draw_arrays(glow::TRIANGLES, 0, 3);
    gl.disable_vertex_attrib_array(0);
    if r.vao.is_some() {
        gl.bind_vertex_array(None);
    }
}

/// Ajoute le filtre sur le calque donné (à appeler après tout le reste).
pub fn paint(ctx: &egui::Context, layer: LayerId, f: &BitmapFilter, gl_res: &BitmapGl, ink: Color32, paper: Color32) {
    if !f.enabled || f.intensity <= 0.0 {
        return;
    }
    let screen: Rect = ctx.screen_rect();
    let f = *f;
    let res = gl_res.0.clone();
    let cb = egui_glow::CallbackFn::new(move |info, painter| {
        let gl = painter.gl();
        let mut slot = res.lock().unwrap();
        if slot.is_none() {
            *slot = Some(unsafe { compile(gl) });
            if let Some(Err(e)) = slot.as_ref() {
                eprintln!("Filtre bitmap indisponible : {e}");
            }
        }
        if let Some(Ok(r)) = slot.as_ref() {
            let [w, h] = info.screen_size_px;
            unsafe { draw(gl, r, &f, w as i32, h as i32, info.pixels_per_point, ink, paper) };
        }
    });
    ctx.layer_painter(layer).add(PaintCallback { rect: screen, callback: Arc::new(cb) });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_saved_settings() {
        let f = BitmapFilter::default();
        assert!(!f.enabled);
        assert_eq!(f.levels, 3);
        assert_eq!(f.mode, BitmapMode::Color);
    }
}
