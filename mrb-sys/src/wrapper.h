#include <mruby.h>
#include <mruby/string.h>
#include <mruby/throw.h>
#include <mruby/value.h>

int
mrbrs_gc_arena_save(mrb_state *mrb);

void
mrbrs_gc_arena_restore(mrb_state *mrb, int idx);

mrb_value
mrbrs_obj_value(void* ptr);

struct RClass*
mrbrs_define_class(mrb_state* mrb, const char* name, struct RClass* superclass, struct RObject** out_exc);

const char*
mrbrs_inspect(mrb_state* mrb, mrb_value obj, size_t* out_len);
