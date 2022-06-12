
use nannou::prelude::*;
use nannou_osc as osc;
use std::{thread, time};

const MONOME_PORT: u16 = 12002;
const MONOME_RX_PORT: u16 = 12288;

struct MonomeGrid {
    device_in_port: Option<i32>,
    device_out_port: Option<i32>,
}

impl MonomeGrid {
    fn new() -> MonomeGrid {
        MonomeGrid {
            device_in_port: None,
            device_out_port: Some(13001),
        }
    }
}

struct Model {
    sender: osc::Sender<osc::Connected>,
    receiver: osc::Receiver,
    received_packets: Vec<(std::net::SocketAddr, osc::Packet)>,

    grid: MonomeGrid,
    grid_sender: Option<osc::Sender<osc::Connected>>,
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
        grid_sender: None,
        led_state: false,
    }
}


fn event(_app: &App, _model: &mut Model, event: Event) {

}


fn update(_app: &App, _model: &mut Model, _update: Update) {
    // get_osc_packets(_model);
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
                find_grid(_model)
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


fn list_monome_devices(_model: &Model) {
    let osc_addr = "/serialosc/list".to_string();
    let args = vec![osc::Type::String("127.0.0.1".to_string()), osc::Type::Int(MONOME_RX_PORT as i32)];
    let packet = (osc_addr, args);
    _model.sender.send(packet).ok();
}


fn find_grid(_model: &mut Model) {
    if _model.received_packets.len() == 0 {
        println!("No received packets.  Sending ping to serialoscd");
        list_monome_devices(_model);

        thread::sleep(time::Duration::from_millis(500));
    }

    get_osc_packets(_model);

    //  Check each packet for the correct address and add a grid instance for each one found
    for &(addr, ref packet) in _model.received_packets.iter() {
        match packet {
            osc::Packet::Message(msg) => {
                if msg.addr.eq("/serialosc/device") {
                    match &msg.args {
                        Some(args) => { 
                            _model.grid.device_in_port = get_monome_listening_port(&args); 

                            let target_addr = format!("{}:{}", "127.0.0.1", _model.grid.device_in_port.clone().unwrap());
                            _model.grid_sender = Some(osc::sender()
                                                        .expect("Could not bind to default socket")
                                                        .connect(target_addr)
                                                        .expect("Could not connect to socket at address"));
                        }
                        None => panic!("Invalid /serialosc/device packet")
                    }
                }
            },
            osc::Packet::Bundle(bundle) => {},
        }
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

    match _model.grid.device_in_port {
        Some(port) => {
            let osc_addr = "/monome/grid/led/all".to_string();
            let args = vec![osc::Type::Int(state_to_write)];
            let packet = (osc_addr, args);

            match &_model.grid_sender {
                Some(sender) => {
                    sender.send(packet).ok();
                },
                None => {},
            }
        },

        None => println!("No grid found yet"),
    }
}
