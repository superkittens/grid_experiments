use nannou::prelude::*;
use nannou_osc as osc;
use std::{thread, time};

const MONOME_PORT: u16 = 12002;
const MONOME_RX_PORT: u16 = 12288;

struct MonomeGrid {
    device_in_port: Option<i32>,
    device_out_port: Option<i32>,
    sender: Option<osc::Sender<osc::Connected>>,
    receiver: Option<osc::Receiver>,

    received_packets: Vec<(std::net::SocketAddr, osc::Packet)>,
    button_latch_state: [u8; 8],
}

impl MonomeGrid {
    fn new() -> MonomeGrid {
        MonomeGrid {
            device_in_port: None,
            device_out_port: Some(13001),
            sender: None,
            receiver: None,
            received_packets: vec![],
            button_latch_state: [0; 8],
        }
    }
}

struct SerialOsc {
    sender: osc::Sender<osc::Connected>,
    receiver: osc::Receiver,
    received_packets: Vec<(std::net::SocketAddr, osc::Packet)>,
}

impl SerialOsc {
    fn new() -> SerialOsc {
        let target_addr = format!("{}:{}", "127.0.0.1", MONOME_PORT);

        let sender = osc::sender()
            .expect("Could not bind to default socket")
            .connect(target_addr)
            .expect("Could not connect to socket at address");

        let receiver = osc::receiver(MONOME_RX_PORT).unwrap();

        SerialOsc {
            sender: sender,
            receiver: receiver,
            received_packets: vec![],
        }
    }
}

struct Model {
    serial_osc: SerialOsc,
    grid: MonomeGrid,
    led_state: bool,
}

fn main() {
    nannou::app(model)
        .event(event)
        .update(update)
        .view(view)
        .run();
}

fn model(app: &App) -> Model {
    app.new_window()
        .size(720, 720)
        .event(window_event)
        .build()
        .unwrap();

    Model {
        serial_osc: SerialOsc::new(),
        grid: MonomeGrid::new(),
        led_state: false,
    }
}

fn event(_app: &App, _model: &mut Model, event: Event) {}

fn update(_app: &App, _model: &mut Model, _update: Update) {
    get_osc_packets(_model);
    get_monome_packets(&mut _model.grid);

    process_serialosc_events(_model);
    process_monome_events(&mut _model.grid);

    light_leds(&_model.grid);
}

fn view(_app: &App, _model: &Model, frame: Frame) {
    frame.clear(SKYBLUE);

    let draw = _app.draw();
    let rect = frame.rect().pad(10.0);

    let mut packets_text = format!("Listening to 127.0.0.1:12288");

    draw.text(&packets_text)
        .font_size(16)
        .align_text_top()
        .line_spacing(10.0)
        .left_justify()
        .wh(rect.wh());

    draw.to_frame(_app, &frame).unwrap();
}

fn window_event(_app: &App, _model: &mut Model, event: WindowEvent) {
    match event {
        KeyPressed(_key) => {
            if let Key::F = _key {
                list_monome_devices(_model);
            }

            if let Key::L = _key {
                if _model.led_state == true {
                    light_all_leds(_model, 1);
                    _model.led_state = false;
                } else {
                    light_all_leds(_model, 0);
                    _model.led_state = true;
                }
            }
        }
        _ => {}
    }
}

fn get_osc_packets(_model: &mut Model) {
    for (packet, addr) in _model.serial_osc.receiver.try_iter() {
        _model.serial_osc.received_packets.push((addr, packet));
    }
}

fn get_monome_packets(grid: &mut MonomeGrid) {
    if let Some(receiver) = grid.receiver.as_ref() {
        for (packet, addr) in receiver.try_iter() {
            grid.received_packets.push((addr, packet));
        }
    }
}

fn process_serialosc_events(_model: &mut Model) {
    for (_socket, packet) in _model.serial_osc.received_packets.iter() {
        match packet {
            osc::Packet::Message(msg) => {
                println!("{:?}", msg.addr);

                if msg.addr.eq("/serialosc/device") {
                    add_monome_device(&_model.serial_osc, &mut _model.grid, msg);

                    if let Some(args) = &msg.args {
                        let serial_number = args[0].clone().string().unwrap();
                        println!("Monome Serial Number: {}", serial_number);
                    }
                }
            }
            osc::Packet::Bundle(_bundle) => {}
        }
    }

    _model.serial_osc.received_packets.clear();
}

fn process_monome_events(grid: &mut MonomeGrid) {
    for (_socket, packet) in grid.received_packets.iter() {
        match packet {
            osc::Packet::Message(msg) => {
                if msg.addr.eq("/monome/grid/key") {
                    if let Some(args) = &msg.args {
                        let x = args[0].clone().int().unwrap() as usize;
                        let y = args[1].clone().int().unwrap() as usize;
                        let state = args[2].clone().int().unwrap();

                        println!("x: {} y: {} s: {}", x, y, state);
                        set_button_latch_state(&mut grid.button_latch_state, x, y, state);
                    }
                }
            }
            _ => {}
        }
    }

    grid.received_packets.clear();
}

fn list_monome_devices(_model: &Model) {
    let osc_addr = "/serialosc/list".to_string();
    let args = vec![
        osc::Type::String("127.0.0.1".to_string()),
        osc::Type::Int(MONOME_RX_PORT as i32),
    ];
    let packet = (osc_addr, args);
    _model.serial_osc.sender.send(packet).ok();
}

fn add_monome_device(serial_osc: &SerialOsc, grid: &mut MonomeGrid, msg: &osc::Message) {
    match &msg.args {
        Some(args) => {
            grid.device_in_port = get_monome_listening_port(&args);

            let target_addr = format!("{}:{}", "127.0.0.1", grid.device_in_port.clone().unwrap());
            grid.sender = Some(
                osc::sender()
                    .expect("Could not bind to default socket")
                    .connect(target_addr)
                    .expect("Could not connect to socket at address"),
            );

            //  Set the monome device out port
            let osc_addr = "/sys/port";
            let args = vec![osc::Type::Int(grid.device_out_port.clone().unwrap())];
            if let Some(sender) = grid.sender.as_ref() {
                sender.send((osc_addr, args)).ok();
            }

            let receiver_port = grid.device_out_port.clone().unwrap();
            grid.receiver = Some(osc::receiver(receiver_port as u16).unwrap());
        }
        None => panic!("Invalid /serialosc/device packet"),
    }
}

fn get_monome_listening_port(args: &Vec<osc::Type>) -> Option<i32> {
    for arg in args.iter() {
        match arg {
            osc::Type::Int(port) => {
                return Some(port.clone());
            }
            _ => {}
        }
    }

    return None;
}

fn set_button_latch_state(latch_states: &mut [u8; 8], x: usize, y: usize, state: i32) {
    if state == 1 {
        let row_state = &mut latch_states[y];

        if *row_state & (1 << x) == 0 {
            *row_state = *row_state | (1 << x);
        } else {
            *row_state = *row_state & ((1 << x) ^ 0xFF);
        }
    }
}

fn light_all_leds(_model: &Model, state: i32) {
    let state_to_write = if state >= 1 { 1 } else { 0 };

    if let Some(sender) = &_model.grid.sender {
        let osc_addr = "/monome/grid/led/all".to_string();
        let args = vec![osc::Type::Int(state_to_write)];
        let packet = (osc_addr, args);

        sender.send(packet).ok();
    }
}

fn light_leds(grid: &MonomeGrid) {
    let mut row = 0;

    for button_states in grid.button_latch_state {
        if let Some(sender) = &grid.sender {
            let osc_addr = "/monome/grid/led/row".to_string();
            let args = vec![osc::Type::Int(0), osc::Type::Int(row), osc::Type::Int(button_states as i32)];

            sender.send((osc_addr, args)).ok();
        }

        row += 1;
    }
}
