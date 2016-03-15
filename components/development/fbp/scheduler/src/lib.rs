#[macro_use]
extern crate rustfbp;
use rustfbp::scheduler::{Comp, Scheduler};

extern crate capnp;

mod contract_capnp {
    include!("fbp_graph.rs");
    include!("path.rs");
    include!("generic_text.rs");
}
use contract_capnp::fbp_graph;
use contract_capnp::path;
use contract_capnp::generic_text;

component! {
    schedulder,
    inputs(input: fbp_graph, contract_path: path, iip: any),
    inputs_array(),
    outputs(error: error, ask_path: path, iip_path: path, iip_contract: generic_text, iip_input: generic_text),
    outputs_array(),
    option(),
    acc(),
    fn run(&mut self) -> Result<()> {

        let mut sched = Scheduler::new();

        // retrieve the asked graph
        let mut ip = try!(self.ports.recv("input"));
        let i_graph: fbp_graph::Reader = try!(ip.get_root());

        for n in try!(i_graph.borrow().get_nodes()).iter() {
            sched.add_component(try!(n.get_name()), try!(n.get_sort()));
        }

        for e in try!(i_graph.borrow().get_edges()).iter() {
            let o_name = try!(e.get_o_name()).into();
            let o_port = try!(e.get_o_port()).into();
            let o_selection: String = try!(e.get_o_selection()).into();
            let i_port = try!(e.get_i_port()).into();
            let i_selection: String = try!(e.get_i_selection()).into();
            let i_name = try!(e.get_i_name()).into();

            match (try!(e.get_o_selection()), try!(e.get_i_selection())) {
                ("", "") => {
                    try!(sched.connect(o_name, o_port, i_name, i_port));
                },
                (_, "") => {
                    try!(sched.add_output_array_selection(o_name.clone(), o_port.clone(), o_selection.clone()));
                    try!(sched.connect_array(o_name, o_port, o_selection, i_name, i_port));
                },
                ("", _) => {
                    try!(sched.soft_add_input_array_selection(i_name.clone(), i_port.clone(), i_selection.clone()));
                    try!(sched.connect_to_array(o_name, o_port, i_name, i_port, i_selection));
                },
                _ => {
                    try!(sched.add_output_array_selection(o_name.clone(), o_port.clone(), o_selection.clone()));
                    try!(sched.soft_add_input_array_selection(i_name.clone(), i_port.clone(), i_selection.clone()));
                    try!(sched.connect_array_to_array(o_name, o_port, o_selection, i_name, i_port, i_selection));
                }
            }
        }

        let (mut p, senders) = try!(Ports::new("exterior".into(), sched.sender.clone(),
                               vec![],
                               vec![],
                               vec!["s".into()],
                               vec![]));
        sched.components.insert("exterior".into(), Comp{
            inputs: senders,
            inputs_array: HashMap::new(),
            sort: "".into(),
        });

        for iip in try!(i_graph.borrow().get_iips()).iter() {

            let comp = try!(iip.get_comp());
            let port = try!(iip.get_port());
            let input = try!(iip.get_iip());

            let (contract, input, option_action) = try!(split_input(input));

            // Get the real path
            let mut new_out = IP::new();
            {
                let mut cont = new_out.init_root::<path::Builder>();
                cont.set_path(&contract);
            }
            try!(self.ports.send("ask_path", new_out));

            let mut contract_path_ip = try!(self.ports.recv("contract_path"));
            let contract_path: path::Reader = try!(contract_path_ip.get_root());

            let c_path = try!(contract_path.get_path());
            let c_path = format!("/nix/store/{}/src/contract.capnp", c_path);
            let contract_camel_case = to_camel_case(&contract);

            if try!(iip.get_selection()) == "" {
                try!(p.connect("s".into(), try!(sched.get_sender(try!(iip.get_comp()).into(), try!(iip.get_port()).into()))));
            } else {
                try!(p.connect("s".into(), try!(sched.get_array_sender(try!(iip.get_comp()).into(), try!(iip.get_port()).into(), try!(iip.get_selection()).into()))));
            }

            let mut new_out = IP::new();
            {
                let mut path = new_out.init_root::<path::Builder>();
                path.set_path(&c_path);
            }
            try!(self.ports.send("iip_path", new_out));

            let mut new_out = IP::new();
            {
                let mut path = new_out.init_root::<generic_text::Builder>();
                path.set_text(&contract_camel_case);
            }
            try!(self.ports.send("iip_contract", new_out));

            let mut new_out = IP::new();
            {
                let mut path = new_out.init_root::<generic_text::Builder>();
                path.set_text(&input);
            }
            try!(self.ports.send("iip_input", new_out));

            let mut iip = try!(self.ports.recv("iip"));
            option_action.map(|action| { iip.action = action; });
            try!(p.send("s", iip));
        }

        sched.join();
        Ok(())
    }
}

fn to_camel_case(s: &str) -> String {
    let mut result = "".to_string();
    for word in s.split("_") {
        result = format!("{}{}", result, capitalize_first_letter(word));
    }
    result
}

fn capitalize_first_letter(s : &str) -> String {
    use std::ascii::*;
    let mut result_chars : Vec<char> = Vec::new();
    for c in s.chars() { result_chars.push(c) }
    result_chars[0] = (result_chars[0] as u8).to_ascii_uppercase() as char;
    return result_chars.into_iter().collect();
}

fn split_input(s: &str) -> Result<(String, String, Option<String>)> {
    let pos = try!(s.find(":").ok_or(result::Error::Misc("bad definition of iip".into())));
    let (a, b) = s.split_at(pos);
    let (_, b) = b.split_at(1);
    let pos2 = b.find("~");
    if let Some(pos) = pos2 {
        let (b, c) = b.split_at(pos);
        let (_, c) = c.split_at(1);
        return Ok((a.into(), b.into(), Some(c.into())));
    };
    Ok((a.into(), b.into(), None))
}
