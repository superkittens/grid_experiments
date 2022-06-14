
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
}

impl MonomeGrid {
    fn new() -> MonomeGrid {
        MonomeGrid {
            device_in_port: None,
            device_out_port: Some(13001),
            sender: None,
            receiver: None,
        }
    }
}

struct Model {
    sender: osc::Sender<osc::Connected>,
    receiver: osc::Receiver,
    received_packets: Vec<(std::net::SocketAddr, osc::Packet)>,

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

    let target_addr = format!("{}:{}", "127.0.0.1", MONOME_PORT);

    let sender = osc::sender()
            .expect("Could not bind to default socket")
            .connect(target_addr)
            .expect("Could not connect to socket at address");

    let receiver = osc::receiver(MONOME_RX_PORT).unwrap();
    let received_packets = vec![];

    Model {
        sender,
        receiver,
        received_packets,
        grid: MonomeGrid::new(),
        led_state: false,
    }
}


fn event(_app: &App, _model: &mut Model, event: Event) {

}


fn update(_app: &App, _model: &mut Model, _update: Update) {
    get_osc_packets(_model);
    process_monome_events(_model);
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
    for (packet, addr) in _model.receiver.try_iter() {
        _model.received_packets.push((addr, packet));
    }
}


fn process_monome_events(_model: &mut Model) {
    for (socket, packet) in _model.received_packets.iter() {
        match packet {
            osc::Packet::Message(msg) => {
                if msg.addr.eq("/serialosc/device") {
                    add_monome_device(&mut _model.grid, msg);

                    if let Some(args) = &msg.args {
                        let serial_number = args[0].clone().string().unwrap();
                        println!("Monome Serial Number: {}", serial_number);
                    }
                }
            },
            osc::Packet::Bundle(bundle) => {},
        }
    }

    _model.received_packets.clear();
}


fn list_monome_devices(_model: &Model) {
    let osc_addr = "/serialosc/list".to_string();
    let args = vec![osc::Type::String("127.0.0.1".to_string()), osc::Type::Int(MONOME_RX_PORT as i32)];
    let packet = (osc_addr, args);
    _model.sender.send(packet).ok();
}


fn add_monome_device(grid: &mut MonomeGrid, msg: &osc::Message) {
    match &msg.args {
         Some(args) => { 
            grid.device_in_port = get_monome_listening_port(&args); 

            let target_addr = format!("{}:{}", "127.0.0.1", grid.device_in_port.clone().unwrap());
            grid.sender = Some(osc::sender()
                                            .expect("Could not bind to default socket")
                                            .connect(target_addr)
                                            .expect("Could not connect to socket at address"));
        },
         None => panic!("Invalid /serialosc/device packet"),
    }
}


fn get_monome_listening_port(args: &Vec<osc::Type>) -> Option<i32> {
    for arg in args.iter() {
        match arg {
            osc::Type::Int(port) => { return Some(port.clone()); }
            _ => {},
        }
    }

    return None;
}

fn light_all_leds(_model: &Model, state: i32) {
    let state_to_write = if state >= 1 {
        1
    } else {
        0
    };

    if let Some(sender) = &_model.grid.sender {
        let osc_addr = "/monome/grid/led/all".to_string();
        let args = vec![osc::Type::Int(state_to_write)];
        let packet = (osc_addr, args);

        sender.send(packet).ok();
    }
}
