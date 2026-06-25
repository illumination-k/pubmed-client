// We need to forward routine registration from C to Rust to avoid the linker
// removing the static library (the Rust object containing the registration
// routine looks unused to the C linker otherwise).

void R_init_pubmedclient_extendr(void *dll);

void R_init_pubmedclient(void *dll) {
    R_init_pubmedclient_extendr(dll);
}
