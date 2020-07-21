#![no_std]
#![no_main]
#![feature(core_intrinsics)]

use core::ffi::c_void;
use core::intrinsics::{fabsf32 as fabsf, maxnumf32 as max, minnumf32 as min};
use core::mem::{size_of, size_of_val};
use psp::math::{cosf32, sinf32};
use psp::sys::{
    self, gum_normalize, ClearBuffer, DepthFunc, DisplayPixelFormat, FrontFaceDirection,
    GuContextType, GuPrimitive, GuState, GuSyncBehavior, GuSyncMode, LightComponent, LightType,
    MatrixMode, ScePspFVector3, ShadingModel, VertexType, GU_PI,
};
use psp::{BUF_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH};

psp::module!("gu_morph", 1, 1);

static mut LIST: psp::Align16<[u32; 0x40000]> = psp::Align16([0; 0x40000]);

#[derive(Debug, Default, Clone, Copy)]
struct Vertex {
    color: u8,
    normal: ScePspFVector3,
    pos: ScePspFVector3,
}

#[derive(Debug, Default, Clone, Copy)]
struct MorphVertex {
    v0: Vertex,
    v1: Vertex,
}

const ROWS: usize = 64;
const COLS: usize = 64;

fn psp_main() {
    psp::enable_home_button();
    unsafe {
        psp_main_inner()
    }
}

unsafe fn psp_main_inner() {

    let mut indices = psp::Align16([0 as usize; (ROWS + 1) * (COLS + 1) * 6]);
    let mut vertices = psp::Align16([MorphVertex::default(); ROWS * COLS]);

    for i in 0..ROWS {
        let di = i as f32 / ROWS as f32;
        let s = di * GU_PI * 2.0;
        let v = ScePspFVector3 {
            x: cosf32(s),
            y: cosf32(s),
            z: sinf32(s),
        };

        for j in 0..COLS {
            let loc = (j + (i * COLS)) * 6;
            let curr = &mut indices.0[loc..loc + 6];
            let (i1, j1) = ((i + 1) % ROWS, (j + 1) % COLS);

            let t = ((j as f32) / COLS as f32) * GU_PI * 2.0;

            let v2 = ScePspFVector3 {
                x: v.x * cosf32(t),
                y: v.y * sinf32(t),
                z: v.z,
            };
            let mut v3 = ScePspFVector3::default();

            // cheap mans sphere -> cube algo :D
            v3.x = if v2.x > 0.0 {
                min(v2.x * 10.0, 1.0)
            } else {
                max(v2.x * 10.0, -1.0)
            };
            v3.y = if v2.y > 0.0 {
                min(v2.y * 10.0, 1.0)
            } else {
                max(v2.y * 10.0, -1.0)
            };
            v3.z = if v2.z > 0.0 {
                min(v2.z * 10.0, 1.0)
            } else {
                max(v2.z * 10.0, -1.0)
            };

            vertices.0[j + i * COLS].v0.color = ((0xff << 24)
                | (((fabsf(v2.x) * 255.0) as u32) << 16)
                | (((fabsf(v2.y) * 255.0) as u32) << 8)
                | ((fabsf(v2.z) * 255.0) as u32))
                as u8;
            vertices.0[j + i * COLS].v0.normal = v2;
            vertices.0[j + i * COLS].v0.pos = v2;

            vertices.0[j + i * COLS].v1.color = vertices.0[j + i * COLS].v0.color;
            vertices.0[j + i * COLS].v1.normal = v3;
            gum_normalize(&mut vertices.0[j + i * COLS].v1.normal);
            vertices.0[j + i * COLS].v1.pos = v3;

            // indices
            curr[0] = j + i * COLS;
            curr[1] = j1 + i * COLS;
            curr[2] = j + i1 * COLS;

            curr[3] = j1 + i * COLS;
            curr[4] = j1 + i1 * COLS;
            curr[5] = j + i1 * COLS;
        }
    }

    // sceKernelDcacheWritebackAll();

    // setup GU

    sys::sceGuInit();

    sys::sceGuStart(GuContextType::Direct, &mut LIST as *mut _ as *mut c_void);
    sys::sceGuDrawBuffer(
        DisplayPixelFormat::Psm8888,
        0 as *const u8 as _,
        BUF_WIDTH as i32,
    );
    sys::sceGuDispBuffer(
        SCREEN_WIDTH as i32,
        SCREEN_HEIGHT as i32,
        0x88000 as *mut c_void,
        BUF_WIDTH as i32,
    );
    sys::sceGuDepthBuffer(0x110000 as *mut c_void, BUF_WIDTH as i32);
    sys::sceGuOffset(2048 - (SCREEN_WIDTH / 2), 2048 - (SCREEN_HEIGHT / 2));
    sys::sceGuViewport(2048, 2048, SCREEN_WIDTH as i32, SCREEN_HEIGHT as i32);
    sys::sceGuDepthRange(0xc350, 0x2710);
    sys::sceGuScissor(0, 0, SCREEN_WIDTH as i32, SCREEN_HEIGHT as i32);
    sys::sceGuEnable(GuState::ScissorTest);
    sys::sceGuDepthFunc(DepthFunc::GreaterOrEqual);
    sys::sceGuEnable(GuState::DepthTest);
    sys::sceGuFrontFace(FrontFaceDirection::Clockwise);
    sys::sceGuShadeModel(ShadingModel::Smooth);
    sys::sceGuEnable(GuState::CullFace);
    sys::sceGuEnable(GuState::Lighting);
    sys::sceGuEnable(GuState::Light0);
    sys::sceGuFinish();
    sys::sceGuSync(GuSyncMode::Finish, GuSyncBehavior::Wait);

    sys::sceDisplayWaitVblankStart();
    sys::sceGuDisplay(true);

    sys::sceGumMatrixMode(MatrixMode::Projection);
    sys::sceGumLoadIdentity();
    sys::sceGumPerspective(75.0, 16.0 / 9.0, 0.5, 1000.0);

    sys::sceGumMatrixMode(MatrixMode::View);
    {
        let pos = ScePspFVector3 {
            x: 0.0,
            y: 0.0,
            z: -2.5,
        };

        sys::sceGumLoadIdentity();
        sys::sceGumTranslate(&pos);
    }

    // run sample

    let mut val = 0.0;

    loop {
        let lpos = ScePspFVector3 {
            x: 1.0,
            y: 0.0,
            z: 1.0,
        };
        sys::sceGuStart(GuContextType::Direct, &mut LIST as *mut _ as *mut c_void);

        // clear screen

        sys::sceGuClearColor(0xff554433);
        sys::sceGuClearDepth(0);
        sys::sceGuLight(
            0,
            LightType::Directional,
            LightComponent::DIFFUSE | LightComponent::SPECULAR,
            &lpos,
        );
        sys::sceGuLightColor(
            0,
            LightComponent::DIFFUSE | LightComponent::SPECULAR,
            0xffffffff,
        );
        sys::sceGuClear(ClearBuffer::COLOR_BUFFER_BIT | ClearBuffer::DEPTH_BUFFER_BIT);
        sys::sceGuSpecular(12.0);

        // rotate morphing mesh

        sys::sceGumMatrixMode(MatrixMode::Model);
        {
            let rot = ScePspFVector3 {
                x: val * 0.79 * (GU_PI / 180.0),
                y: val * 0.98 * (GU_PI / 180.0),
                z: val * 1.32 * (GU_PI / 180.0),
            };

            sys::sceGumLoadIdentity();
            sys::sceGumRotateXYZ(&rot);
        }

        sys::sceGuAmbientColor(0xffffffff);

        // draw cube

        sys::sceGuMorphWeight(0, 0.5 * sinf32(val * (GU_PI / 180.0)) + 0.5);
        sys::sceGuMorphWeight(1, -0.5 * sinf32(val * (GU_PI / 180.0)) + 0.5);
        sys::sceGumDrawArray(
            GuPrimitive::Triangles,
            VertexType::COLOR_8888
                | VertexType::NORMAL_32BITF
                | VertexType::VERTEX_32BITF
                | VertexType::VERTICES2
                | VertexType::INDEX_16BIT
                | VertexType::TRANSFORM_3D,
            (size_of_val(&indices) / size_of::<usize>()) as i32,
            &mut indices as *const _ as *mut c_void,
            &mut vertices as *const _ as *mut c_void,
        );

        sys::sceGuFinish();
        sys::sceGuSync(GuSyncMode::Finish, GuSyncBehavior::Wait);

        sys::sceDisplayWaitVblankStart();
        sys::sceGuSwapBuffers();

        val += 1.0;
    }

    //sys::sceGuTerm();

    //sys::sceKernelExitGame();
}
