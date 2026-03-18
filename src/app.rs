/// The app module owns the window, rendering loop, and base surface that all
/// other graphical elements are arranged on.
use std::{iter, sync::Arc, vec};

use crate::frame::Frame;
use crate::layout::PlotOutput;
use crate::column::PlotData;
use crate::plot::Blueprint;
use crate::theme::Theme;
use glyph_brush::ab_glyph::FontRef;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::frame::{text_projection_matrix, ViewUniform};
use crate::shape::TextRotation;
use std::collections::HashMap;
use wgpu_text::{BrushBuilder, TextBrush};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct AppState<'a> {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    window: Arc<Window>,
    frame: Option<Frame>,
    brushes: HashMap<TextRotation, TextBrush<FontRef<'a>>>,
    plot_output: PlotOutput,
    theme: Theme,
    view_uniform: ViewUniform,
}

impl AppState<'_> {
    async fn new(
        window: Arc<Window>,
        plot_output: PlotOutput,
        theme: Theme,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        let font = include_bytes!("fonts/OpenSans-Regular.ttf");
        let w = config.width as f32;
        let h = config.height as f32;
        let mut brushes = HashMap::new();
        for rotation in [TextRotation::None, TextRotation::Ccw90, TextRotation::Cw90] {
            let brush = BrushBuilder::using_font_bytes(font)
                .unwrap()
                .with_matrix(text_projection_matrix(rotation, w, h))
                .build(&device, config.width, config.height, config.format);
            brushes.insert(rotation, brush);
        }

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            frame: None,
            brushes,
            plot_output,
            theme,
            view_uniform: ViewUniform::identity(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
            let w = self.config.width as f32;
            let h = self.config.height as f32;
            for (&rotation, brush) in self.brushes.iter_mut() {
                brush.update_matrix(text_projection_matrix(rotation, w, h), &self.queue);
            }
            self.update()
        }
    }

    fn update(&mut self) {
        let frame = Frame::new(
            &self.device,
            &self.config,
            self.window.clone(),
            &self.queue,
            &mut self.brushes,
            &self.plot_output,
            &self.theme,
            self.view_uniform,
        );
        self.frame = Some(frame);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.,
                            g: 1.,
                            b: 1.,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            if let Some(frame) = &mut self.frame {
                frame.render(&mut render_pass);
            }

            for brush in self.brushes.values_mut() {
                brush.draw(&mut render_pass);
            }
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<AppState<'static>>>,
    state: Option<AppState<'static>>,
    pending_plot_output: Option<PlotOutput>,
    pending_theme: Option<Theme>,
}

impl App {
    pub fn new(
        plot_output: PlotOutput,
        theme: Theme,
        #[cfg(target_arch = "wasm32")] event_loop: &EventLoop<AppState<'static>>,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            pending_plot_output: Some(plot_output),
            pending_theme: Some(theme),
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

impl ApplicationHandler<AppState<'static>> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let plot_output = self.pending_plot_output.take().unwrap();
        let theme = self.pending_theme.take().unwrap_or_default();

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state =
                Some(pollster::block_on(AppState::new(window, plot_output, theme)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy
                        .send_event(
                            AppState::new(window, plot_output, theme)
                                .await
                                .expect("Unable to create canvas!!!")
                        )
                        .is_ok())
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: AppState<'static>) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match (button, state.is_pressed()) {
                (MouseButton::Left, true) => {}
                (MouseButton::Left, false) => {}
                _ => {}
            },
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}

pub fn run(mut blueprint: Blueprint, data: PlotData) -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let plot_output = blueprint.render(data).map_err(|e| anyhow::anyhow!(e))?;
    let theme = Theme::default();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new(
        plot_output,
        theme,
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    unimplemented!("WASM target requires data to be passed via JS interop");
}
