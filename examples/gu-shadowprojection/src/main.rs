#![no_std]
#![no_main]
#![feature(core_intrinsics)]


use core::mem;
use core::intrinsics;
use core::ffi::c_void;
use psp::sys::{self, 
    GuSyncMode, GuSyncBehavior,LightType,LightComponent,TextureMapMode,GuTexWrapMode,
    FrontFaceDirection, MatrixMode, GuPrimitive, TexturePixelFormat, ScePspFMatrix4, TextureFilter,
    ShadingModel,ClearBuffer, TextureProjectionMapMode,MipmapLevel, TextureColorComponent, TextureEffect,
    VertexType, ScePspFVector3, GU_PI, GuContextType, DisplayPixelFormat, GuState, DepthFunc
};
use psp::vram_alloc::SimpleVramAllocator;
use psp::{BUF_WIDTH, SCREEN_WIDTH, SCREEN_HEIGHT};

psp::module!("gu_shadowprojection", 1, 1);

const TWO_PI: f32 = 2.0 * GU_PI;

// TODO: should be i16?
#[derive(Copy, Clone, Debug, Default)]
struct VertexNormal
{
    nx: f32,
    ny: f32,
    nz: f32,
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Copy, Clone, Debug)]
struct Texture
{
    format: TexturePixelFormat,
    mipmap: MipmapLevel,
    width: i32,
    height: i32,
    stride: i32,
    data: *mut c_void,
}

/* grid */
const GRID_COLUMNS: u32 = 32;
const GRID_ROWS: u32 = 32;
const GRID_SIZE: f32 = 10.0;


/* torus */

const TORUS_SLICES: u32 = 48; // numc
const TORUS_ROWS: u32 = 48; // numt
const TORUS_RADIUS: f32 = 1.0;
const TORUS_THICKNESS: f32 = 0.5;


const LIGHT_DISTANCE: f32 = 3.0;

const TEXTURE_FORMAT: TexturePixelFormat = TexturePixelFormat::Psm8888;
//#define PIXEL_SIZE (4) /* change this if you change to another screenmode */
//#define FRAME_SIZE (BUF_WIDTH * SCREEN_HEIGHT * PIXEL_SIZE)
//#define ZBUF_SIZE (BUF_WIDTH SCREEN_HEIGHT * 2) /* zbuffer seems to be 16-bit? */

struct Geometry
{
    world: ScePspFMatrix4,
    count: i32,
    indices: *mut c_void, //&[u16],
    vertices: *mut c_void, //&[VertexNormal],
    color: u32,
}

unsafe fn drawGeometry( geom: &Geometry ) {
    sys::sceGuSetMatrix(MatrixMode::Model,&geom.world);

    sys::sceGuColor(geom.color);
    //sys::sceGuDrawArray(GU_TRIANGLES,GU_NORMAL_32BITF|GU_VERTEX_32BITF|GU_INDEX_16BIT|GU_TRANSFORM_3D,geom.count,geom.indices,geom.vertices);
    sys::sceGuDrawArray(GuPrimitive::Triangles, VertexType::NORMAL_32BITF|VertexType::VERTEX_32BITF|VertexType::INDEX_16BIT|VertexType::TRANSFORM_3D,geom.count,geom.indices as _, geom.vertices as _);
}

unsafe fn drawShadowCaster(geom: &Geometry) {
    sys::sceGuSetMatrix(MatrixMode::Model, &geom.world);

    sys::sceGuColor(0x00000000);
    sys::sceGuDrawArray(GuPrimitive::Triangles,VertexType::NORMAL_32BITF|VertexType::VERTEX_32BITF|VertexType::INDEX_16BIT|VertexType::TRANSFORM_3D,geom.count,geom.indices as _,geom.vertices as _);
}

unsafe fn drawShadowReceiver(geom: &Geometry, mut shadowProjMatrix: ScePspFMatrix4) {
    sys::sceGuSetMatrix(MatrixMode::Model,&geom.world);

    // multiply shadowmap projection texture by geometry world matrix
    // since geometry coords are in object space
    let shadclone = shadowProjMatrix.clone();

    sys::gumMultMatrix(&mut shadowProjMatrix, &shadclone, &geom.world);
    sys::sceGuSetMatrix(MatrixMode::Texture, &shadowProjMatrix);

    sys::sceGuColor(geom.color);
    sys::sceGuDrawArray(GuPrimitive::Triangles,VertexType::NORMAL_32BITF|VertexType::VERTEX_32BITF|VertexType::INDEX_16BIT|VertexType::TRANSFORM_3D,geom.count,geom.indices as _,geom.vertices as _);
}

fn psp_main() {
    unsafe { psp_main_inner() }
}

unsafe fn psp_main_inner() {
    //SetupCallbacks();

    let mut list = psp::Align16([0; 0x40000]);

    let mut grid_vertices = psp::Align16([VertexNormal::default(); (GRID_COLUMNS * GRID_ROWS) as usize]);
    let mut grid_indices = psp::Align16([0u16; ((GRID_COLUMNS-1) * (GRID_ROWS-1) * 6) as usize]);
    //VertexNormal __attribute__((aligned(16))) grid_vertices[GRID_COLUMNS*GRID_ROWS];
    //unsigned short __attribute__((aligned(16))) grid_indices[(GRID_COLUMNS-1)*(GRID_ROWS-1)*6];
    
    let mut torus_vertices = psp::Align16([VertexNormal::default(); (TORUS_SLICES * TORUS_ROWS) as usize]);
    let mut torus_indices = psp::Align16([0u16; (TORUS_SLICES * TORUS_ROWS * 6) as usize]);

    //static mut TORUS_VERTICIES: psp::Align16<[VertexNormal; TORUS_SLICES * TORUS_ROWS]> = psp::Align16([0; TORUS_COLUMNS * TORUS_ROWS]);
    //static mut TORUS_INDICES: psp::Align16<[Texture; TORUS_SLICES * TORUS_ROWS * 6]> = psp::Align16([0; TORUS_COLUMNS * TORUS_ROWS * 6]);
    // generate geometry

    genGrid(GRID_ROWS, GRID_COLUMNS, GRID_SIZE, &mut grid_vertices.0, &mut grid_indices.0 );
    genTorus(TORUS_ROWS, TORUS_SLICES, TORUS_RADIUS, TORUS_THICKNESS, &mut torus_vertices.0, &mut torus_indices.0);

    // flush cache so that no stray data remains

    sys::sceKernelDcacheWritebackAll();

    // setup VRAM buffers

        let mut allocator = SimpleVramAllocator::new();
        let mut frameBuffer = allocator.alloc_texture_pixels(BUF_WIDTH, SCREEN_HEIGHT, TexturePixelFormat::Psm8888).start_addr();

        let doubleBuffer = allocator.alloc_texture_pixels(BUF_WIDTH, SCREEN_HEIGHT, TexturePixelFormat::Psm8888).start_addr();

        // TODO: determine appropriate size of these
        let renderTarget = allocator.alloc_texture_pixels(BUF_WIDTH, SCREEN_HEIGHT, TexturePixelFormat::Psm8888).start_addr();
        let depthBuffer = allocator.alloc_texture_pixels(BUF_WIDTH, SCREEN_HEIGHT, TexturePixelFormat::Psm4444).start_addr();

    //void* frameBuffer = (void*)0;
    //const void* doubleBuffer = (void*)0x44000;
    //const void* renderTarget = (void*)0x88000;
    //const void* depthBuffer = (void*)0x110000;


    // setup GU

    sys::sceGuInit();

    sys::sceGuStart(GuContextType::Direct, &mut list as *mut _ as *mut c_void);
    sys::sceGuDrawBuffer(DisplayPixelFormat::Psm4444,frameBuffer,BUF_WIDTH as i32);
    sys::sceGuDispBuffer(SCREEN_WIDTH as i32,SCREEN_HEIGHT as i32,doubleBuffer as _,BUF_WIDTH as i32);
    sys::sceGuDepthBuffer(depthBuffer as _,BUF_WIDTH as i32);
    sys::sceGuOffset(2048 - (SCREEN_WIDTH/2),2048 - (SCREEN_HEIGHT/2));
    sys::sceGuViewport(2048,2048,SCREEN_WIDTH as i32,SCREEN_HEIGHT as i32);
    sys::sceGuDepthRange(0xc350,0x2710);
    sys::sceGuScissor(0,0,SCREEN_WIDTH as i32,SCREEN_HEIGHT as i32);
    sys::sceGuEnable(GuState::ScissorTest);
    sys::sceGuDepthFunc(DepthFunc::GreaterOrEqual);
    sys::sceGuEnable(GuState::DepthTest);
    sys::sceGuFrontFace(FrontFaceDirection::Clockwise);
    sys::sceGuShadeModel(ShadingModel::Smooth);
    sys::sceGuEnable(GuState::CullFace);
    sys::sceGuEnable(GuState::Texture2D);
    sys::sceGuEnable(GuState::Dither);
    sys::sceGuFinish();
    sys::sceGuSync(GuSyncMode::Finish,GuSyncBehavior::Wait);

    sys::sceDisplayWaitVblankStart();
    sys::sceGuDisplay(true);


    // setup matrices

    let mut identity = ScePspFMatrix4::default();
    let mut projection = ScePspFMatrix4::default();
    let mut view = ScePspFMatrix4::default();

    sys::gumLoadIdentity(&mut identity);

    //sys::gumLoadIdentity(&projection);
    sys::gumLoadIdentity(&mut projection);

    sys::gumPerspective(&mut projection,75.0,16.0/9.0,0.5,1000.0);

    {
        let pos = ScePspFVector3{x: 0.0,y: 0.0,z: -5.0};

        sys::gumLoadIdentity(&mut view);
        sys::gumTranslate(&mut view, &pos);
    }

    let mut textureProjScaleTrans = ScePspFMatrix4::default();
    sys::gumLoadIdentity(&mut textureProjScaleTrans);
    textureProjScaleTrans.x.x = 0.5;
    textureProjScaleTrans.y.y = -0.5;
    textureProjScaleTrans.w.x = 0.5;
    textureProjScaleTrans.w.y = 0.5;

    let mut lightProjection = ScePspFMatrix4::default();
    let mut lightProjectionInf = ScePspFMatrix4::default();
    let mut lightView = ScePspFMatrix4::default();
    let mut lightMatrix = ScePspFMatrix4::default();

    sys::gumLoadIdentity(&mut lightProjection);
    sys::gumPerspective(&mut lightProjection,75.0,1.0,0.1,1000.0);
    sys::gumLoadIdentity(&mut lightProjectionInf);
    sys::gumPerspective(&mut lightProjectionInf,75.0,1.0,0.0,1000.0);

    sys::gumLoadIdentity(&mut lightView);
    sys::gumLoadIdentity(&mut lightMatrix);

    // define shadowmap

    let shadowmap = Texture {
        format: TexturePixelFormat::Psm4444,
                mipmap:    MipmapLevel::None, 
                width:     128, 
                height:       128, 
                stride:     128,
                data: renderTarget
    };

    // define geometry

    let mut torus = Geometry {
        world: identity,
        //count: sizeof(torus_indices)/sizeof(unsigned short),
        count: (mem::size_of_val(&torus_indices)/mem::size_of::<u16>()) as i32,
        indices: &mut torus_indices as *mut _ as *mut c_void,
        vertices: &mut torus_vertices as *mut _ as *mut c_void,
        color: 0xffffff,
    };

    let mut grid = Geometry {
        world: identity,
        //count: sizeof(grid_indices)/sizeof(unsigned short),
        count: (mem::size_of_val(&grid_indices)/mem::size_of::<u16>()) as i32,
        indices: &mut grid_indices as *mut _ as *mut c_void,
        vertices: &mut grid_vertices as *mut _ as *mut c_void,
        color: 0xff7777,
    };

    // run sample

    let mut val = 0;

        loop {
        // update matrices

        // grid
        {
            let pos = ScePspFVector3 {x: 0.0, y: -1.5, z: 0.0};

            sys::gumLoadIdentity(&mut grid.world);
            sys::gumTranslate(&mut grid.world,&pos);
        }

        // torus
        {
            let pos = ScePspFVector3 {x: 0.0, y: 0.5, z: 0.0};
            let rot = ScePspFVector3 {
                x: val as f32 * 0.79 * (GU_PI/180.0), 
                y: val as f32 * 0.98 * (GU_PI/180.0), 
                z: val as f32 * 1.32 * (GU_PI/180.0)
            };

            sys::gumLoadIdentity(&mut torus.world);
            sys::gumTranslate(&mut torus.world,&pos);
            sys::gumRotateXYZ(&mut torus.world,&rot);
        }

        // orbiting light
        {
            let lightLookAt = ScePspFVector3 {x: torus.world.w.x,y: torus.world.w.y,z: torus.world.w.z };
            let rot1 = ScePspFVector3 {x:0.0,y:val as f32 * 0.79 * (GU_PI/180.0),z:0.0};
            let rot2 = ScePspFVector3 {x:-(GU_PI/180.0)*60.0,y:0.0,z:0.0};
            let pos = ScePspFVector3 {x:0.0,y:0.0,z:LIGHT_DISTANCE};

            sys::gumLoadIdentity(&mut lightMatrix);
            sys::gumTranslate(&mut lightMatrix,&lightLookAt);
            sys::gumRotateXYZ(&mut lightMatrix,&rot1);
            sys::gumRotateXYZ(&mut lightMatrix,&rot2);
            sys::gumTranslate(&mut lightMatrix,&pos);
        }

        sys::gumFastInverse(&mut lightView,&lightMatrix);

        // render to shadow map

        {
            sys::sceGuStart(GuContextType::Direct, &mut list as *mut _ as *mut c_void);

            // set offscreen texture as a render target

            sys::sceGuDrawBufferList(DisplayPixelFormat::Psm4444,renderTarget,shadowmap.stride as i32);

            // setup viewport    

            sys::sceGuOffset(2048 - (shadowmap.width/2) as u32,2048 - (shadowmap.height/2) as u32);
            sys::sceGuViewport(2048,2048,shadowmap.width,shadowmap.height);

            // clear screen

            sys::sceGuClearColor(0xffffffff);
            sys::sceGuClearDepth(0);
            sys::sceGuClear(ClearBuffer::COLOR_BUFFER_BIT|ClearBuffer::DEPTH_BUFFER_BIT);

            // setup view/projection from light

            sys::sceGuSetMatrix(MatrixMode::Projection,&lightProjection);
            sys::sceGuSetMatrix(MatrixMode::View,&lightView);

            // shadow casters are drawn in black
            // disable lighting and texturing

            sys::sceGuDisable(GuState::Lighting);
            sys::sceGuDisable(GuState::Texture2D);

            // draw torus to shadow map

            drawShadowCaster( &torus );

            sys::sceGuFinish();
    sys::sceGuSync(GuSyncMode::Finish,GuSyncBehavior::Wait);
        }

        // render to frame buffer

        {
            sys::sceGuStart(GuContextType::Direct,&mut list as *mut _ as *mut c_void);

            // set frame buffer

            sys::sceGuDrawBufferList(DisplayPixelFormat::Psm4444,frameBuffer,BUF_WIDTH as i32);

            // setup viewport

            sys::sceGuOffset(2048 - (SCREEN_WIDTH/2),2048 - (SCREEN_HEIGHT/2));
            sys::sceGuViewport(2048,2048,SCREEN_WIDTH as i32,SCREEN_HEIGHT as i32);
            
            // clear screen

            sys::sceGuClearColor(0xff554433);
            sys::sceGuClearDepth(0);
            sys::sceGuClear(ClearBuffer::COLOR_BUFFER_BIT|ClearBuffer::DEPTH_BUFFER_BIT);

            // setup view/projection from camera

            sys::sceGuSetMatrix(MatrixMode::Projection,&projection);
            sys::sceGuSetMatrix(MatrixMode::View,&view);
            sys::sceGuSetMatrix(MatrixMode::Model,&identity);

            // setup a light
            let lightPos = ScePspFVector3 { x: lightMatrix.w.x, y: lightMatrix.w.y, z: lightMatrix.w.z };
            let lightDir = ScePspFVector3 { x: lightMatrix.z.x, y: lightMatrix.z.y, z: lightMatrix.z.z };

            sys::sceGuLight(0,LightType::Spotlight,LightComponent::DIFFUSE,&lightPos);
            sys::sceGuLightSpot(0,&lightDir, 5.0, 0.6);
            sys::sceGuLightColor(0,LightComponent::DIFFUSE,0x00ff4040);
            sys::sceGuLightAtt(0,1.0,0.0,0.0);
            sys::sceGuAmbient(0x00202020);
            sys::sceGuEnable(GuState::Lighting);
            sys::sceGuEnable(GuState::Light0);

            // draw torus

            drawGeometry( &torus );

            // setup texture projection

            sys::sceGuTexMapMode( TextureMapMode::TextureMatrix, 0, 0 );
            sys::sceGuTexProjMapMode( TextureProjectionMapMode::Position );

            // set shadowmap as a texture

            sys::sceGuTexMode(shadowmap.format,0,0,0);
            sys::sceGuTexImage(shadowmap.mipmap,shadowmap.width,shadowmap.height,shadowmap.stride,shadowmap.data);
            sys::sceGuTexFunc(TextureEffect::Modulate,TextureColorComponent::Rgb);
            sys::sceGuTexFilter(TextureFilter::Linear,TextureFilter::Linear);
            sys::sceGuTexWrap(GuTexWrapMode::Clamp,GuTexWrapMode::Clamp);
            sys::sceGuEnable(GuState::Texture2D);

            // calculate texture projection matrix for shadowmap
 
            let mut shadowProj = ScePspFMatrix4::default();
            sys::gumMultMatrix(&mut shadowProj, &lightProjectionInf, &lightView);
            let shadclone = shadowProj.clone();
            sys::gumMultMatrix(&mut shadowProj, &textureProjScaleTrans, &shadclone);

            // draw grid receiving shadow

            drawShadowReceiver( &grid, shadowProj );

            sys::sceGuFinish();
    sys::sceGuSync(GuSyncMode::Finish,GuSyncBehavior::Wait);
        }

        sys::sceDisplayWaitVblankStart();
        frameBuffer = sys::sceGuSwapBuffers();

        val += 1;
    }
}

/* usefull geometry functions */
fn genGrid(rows: u32, columns: u32, size: f32, dstVertices: &mut [VertexNormal], dstIndices: &mut [u16] )
    //dstIndices: unsigned short* 
{
    // generate grid (TODO: tri-strips)
        for j in 0..rows {
            for i in 0..columns {
                let mut curr = &mut dstVertices[(i+j*columns) as usize];

                curr.nx = 0.0;
                curr.ny = 1.0;
                curr.nz = 0.0;

                curr.x = ((i as f32 * (1.0/(columns as f32)))-0.5) * size as f32;
                curr.y = 0.0;
                curr.z = ((j as f32 * (1.0/(columns as f32)))-0.5) * size as f32;
            }
    }

        for j in 0..(rows-1) {
            for i in 0..(columns-1) {
                let index = ((i+(j*(columns-1)))*6) as usize;
                let curr = &mut dstIndices[index..index+6];

                curr[0] = (i + j * columns) as u16;
                curr[1] = ((i+1) + j * columns) as u16;
                curr[2] = (i + (j+1) * columns) as u16;

                curr[3] = ((i+1) + j * columns) as u16;
                curr[4] = ((i+1) + (j+1) * columns) as u16;
                curr[5] = (i + (j + 1) * columns) as u16;
            }
    }
}

fn genTorus(slices: u32, rows: u32, radius: f32, thickness: f32, dstVertices: &mut [VertexNormal], dstIndices: &mut [u16])
{
    // generate torus (TODO: tri-strips)
        for j in 0..slices {
            for i in 0..rows {
                let curr = &mut dstVertices[(i+j*rows) as usize];
                let s = i as f32 + 0.5;
                let t = j as f32;
                

                unsafe {
                let cs = intrinsics::cosf32(s * TWO_PI/slices as f32);
                let ct = intrinsics::cosf32(t * TWO_PI/rows as f32);
                let ss = intrinsics::sinf32(s * TWO_PI/slices as f32);
                let st = intrinsics::sinf32(t * TWO_PI/rows as f32);

                curr.nx = cs * ct;
                curr.ny = cs * st;
                curr.nz = ss;

                curr.x = (radius + thickness * cs) * ct;
                curr.y = (radius + thickness * cs) * st;
                curr.z = thickness * ss;
                }
            }
    }

        for j in 0..slices {
            for i in 0..rows {
                    let index = ((i+(j*rows))*6) as usize;
                    let curr = &mut dstIndices[index..index+6];
                    let i1 = (i+1)%rows;
                    let j1 = (j+1)%slices;

                    curr[0]  = (i + j * rows) as u16;
                    curr[1]  = (i1 + j * rows) as u16;
                    curr[2]  = (i + j1 * rows) as u16;
                           
                    curr[3]  = (i1 + j * rows) as u16;
                    curr[4]  = (i1 + j1 * rows) as u16;
                    curr[5]  = (i + j1 * rows) as u16;
        }
    }
}
