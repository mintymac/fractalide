{ stdenv, buildFractalideSubnet, upkeepers
  , fs_file_open
  , development_fbp_parser_lexical
  , development_fbp_parser_semantic
  , development_fbp_vm
  , development_fbp_errors
  , development_fbp_parser_print_graph
  , development_fbp_parser_check_graph
  , development_fbp_scheduler
  , development_capnp_encode
  , contract_lookup
  , ...}:
  let
  doc = import ../../../doc {};
  in
  buildFractalideSubnet rec {
   src = ./.;
   subnet = ''
   // Basic output
   open(${fs_file_open}) output -> input lex(${development_fbp_parser_lexical})
   lex() output -> input sem(${development_fbp_parser_semantic})
   sem() output -> input check(${development_fbp_parser_check_graph})
   check() output -> input vm(${development_fbp_vm})

   open() error -> file_error errors(${development_fbp_errors})
   sem() error -> semantic_error errors()
   check() error -> semantic_error errors()

   errors() output -> input vm()

   vm() ask_graph -> input open()

   // Send to sched
   vm() output -> graph sched(${development_fbp_scheduler})
   sched() ask_path -> input contract_lookup(${contract_lookup})
   contract_lookup() output -> contract_path sched()

   sched() ask_graph -> input vm()

   // IIP
   sched() iip_path -> path iip(${development_capnp_encode})
   sched() iip_contract -> contract iip()
   sched() iip_input -> input iip()
   iip() output -> iip sched()

   sched() outputs[test] -> input cl(${fs_file_open})

   action => action sched()
   sched() outputs => outputs
   '';

   meta = with stdenv.lib; {
    description = "Subnet: development testing file";
    homepage = https://github.com/fractalide/fractalide/tree/master/components/development/test;
    license = with licenses; [ mpl20 ];
    maintainers = with upkeepers; [ dmichiels sjmackenzie];
  };
}
