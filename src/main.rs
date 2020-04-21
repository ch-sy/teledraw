#![windows_subsystem = "windows"]
#[macro_use]
extern crate glium;

use std::thread;

use std::net::{TcpListener, TcpStream};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::sync::{Arc, Mutex};
use std::io::{BufWriter, BufReader, Write};
use std::io::BufRead;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

fn handle_client_input(mut stream_in: BufReader<TcpStream>, shape: Arc<Mutex<Vec<Vertex>>>) {
    loop {
        let old_pos_x = stream_in.read_f32::<LittleEndian>().unwrap();
        let old_pos_y = stream_in.read_f32::<LittleEndian>().unwrap();
        let new_pos_x = stream_in.read_f32::<LittleEndian>().unwrap();
        let new_pos_y = stream_in.read_f32::<LittleEndian>().unwrap();
        {
            let mut shape = shape.lock().unwrap();
            shape.push(Vertex { position: [old_pos_x, old_pos_y] });
            shape.push(Vertex { position: [new_pos_x, new_pos_y] });
        }
    }
}

fn main() {
    #[allow(unused_imports)]
    use glium::{glutin, Surface};

    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new();
    let cb = glutin::ContextBuilder::new().with_multisampling(4);
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();



    implement_vertex!(Vertex, position);

    let mut stream = TcpStream::connect("78.47.128.141:2020").unwrap();

    let mut stream_in = BufReader::new(stream.try_clone().unwrap());

    //let mut stream_out = stream;
    let mut stream_out = BufWriter::new(stream);

    let mut shape_size = 0;
    let mut shape = Arc::new(Mutex::new(vec![]));
    {
        let mut shape = shape.lock().unwrap();
        let count = stream_in.read_u32::<LittleEndian>().unwrap() as usize;
        for _ in 0..count {
            shape.push(Vertex { position: [stream_in.read_f32::<LittleEndian>().unwrap(),
                stream_in.read_f32::<LittleEndian>().unwrap()] });
            shape.push(Vertex { position: [stream_in.read_f32::<LittleEndian>().unwrap(),
                stream_in.read_f32::<LittleEndian>().unwrap()] })
        }
    }

    let mut vertex_buffer = {
        let mut shape = shape.lock().unwrap();
        glium::VertexBuffer::new(&display, &shape).unwrap()
    };
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);

    let vertex_shader_src = r#"
        #version 140
        in vec2 position;
        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140
        out vec4 color;
        void main() {
            color = vec4(0.0, 0.0, 0.0, 1.0);
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

    let mut old_position:(f32,f32) = (0.0,0.0);
    let mut button_down = false;

    //let mut out_stream = stream.try_clone().unwrap();
    let mut c_shape = shape.clone();
    thread::spawn(|| {
        handle_client_input(stream_in, c_shape);
    });

    event_loop.run(move |event, _, control_flow| {
        let next_frame_time = std::time::Instant::now() +
            std::time::Duration::from_nanos(16_666_667);
        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);

        match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                },
                glutin::event::WindowEvent::MouseInput{device_id, state, button, modifiers} => {
                    button_down = !button_down;
                },
                glutin::event::WindowEvent::CursorMoved{device_id, position,
                    modifiers} =>  {
                    let new_position = ((position.x as f32 /
                        display.get_framebuffer_dimensions().0 as f32)
                        *2.0-1.0,
                        (position.y as f32 /
                            display.get_framebuffer_dimensions().1 as f32)
                            *(-2.0)+1.0);
                    if button_down {
                        stream_out.write_f32::<LittleEndian>(old_position.0).unwrap();
                        stream_out.write_f32::<LittleEndian>(old_position.1).unwrap();
                        stream_out.write_f32::<LittleEndian>(new_position.0).unwrap();
                        stream_out.write_f32::<LittleEndian>(new_position.1).unwrap();
                        let mut shape = shape.lock().unwrap();
                        shape.push(Vertex { position: [old_position.0, old_position.1] });
                        shape.push(Vertex { position: [new_position.0, new_position.1] });
                    }
                    old_position = new_position;
                },
                glutin::event::WindowEvent::Touch(touch) => {
                    let new_position = ((touch.location.x as f32 /
                        display.get_framebuffer_dimensions().0 as f32)
                                            *2.0-1.0,
                                        (touch.location.y as f32 /
                                            display.get_framebuffer_dimensions().1 as f32)
                                            *(-2.0)+1.0);
                    match touch.phase {
                        glutin::event::TouchPhase::Started => {
                            old_position = new_position;
                        }
                        glutin::event::TouchPhase::Moved => {
                            stream_out.write_f32::<LittleEndian>(old_position.0).unwrap();
                            stream_out.write_f32::<LittleEndian>(old_position.1).unwrap();
                            stream_out.write_f32::<LittleEndian>(new_position.0).unwrap();
                            stream_out.write_f32::<LittleEndian>(new_position.1).unwrap();
                            let mut shape = shape.lock().unwrap();
                            shape.push(Vertex { position: [old_position.0, old_position.1] });
                            shape.push(Vertex { position: [new_position.0, new_position.1] });
                        },
                        _ => return,
                    }
                    old_position = new_position;
                },
                _ => return,
            },
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glutin::event::StartCause::Init => (),
                _ => return,
            },
            _ => return,
        }

        {
            let mut shape = shape.lock().unwrap();
            if shape_size != shape.len() {
                shape_size = shape.len();
                vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
            }
        }
        stream_out.flush();

        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);
        target.draw(&vertex_buffer, &indices, &program, &glium::uniforms::EmptyUniforms,
                    &Default::default()).unwrap();
        target.finish().unwrap();
    });
}