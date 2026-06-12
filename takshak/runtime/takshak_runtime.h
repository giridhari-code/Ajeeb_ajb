// takshak runtime — forward declarations for C-level functions
// All parameters use intptr_t to match Ajeeb's type model
// (strings are passed as intptr_t = char* cast to integer)

#include <stdint.h>

void _takshak_init(void);
void _takshak_get(intptr_t path, intptr_t handler_fn);
void _takshak_post(intptr_t path, intptr_t handler_fn);
void _takshak_listen(intptr_t port);
intptr_t _takshak_req_path(intptr_t req_id);
intptr_t _takshak_req_method(intptr_t req_id);
intptr_t _takshak_req_body(intptr_t req_id);
void _takshak_res_send(intptr_t res_id, intptr_t body);
void _takshak_res_json(intptr_t res_id, intptr_t json);
void _takshak_res_html(intptr_t res_id, intptr_t html);
void _takshak_res_status(intptr_t res_id, intptr_t code);
void _takshak_res_header(intptr_t res_id, intptr_t key, intptr_t val);
