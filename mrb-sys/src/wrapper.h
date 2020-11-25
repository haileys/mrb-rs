#include <mruby.h>
#include <mruby/class.h>
#include <mruby/data.h>
#include <mruby/error.h>
#include <mruby/proc.h>
#include <mruby/string.h>
#include <mruby/throw.h>
#include <mruby/value.h>

typedef struct {
    struct RObject* panic_carrier;
    void* panic_info;
} mrbrs_ud;

mrb_state*
mrbrs_open_core();

void
mrbrs_close(mrb_state* mrb);

int
mrbrs_gc_arena_save(mrb_state *mrb);

void
mrbrs_gc_arena_restore(mrb_state *mrb, int idx);

mrb_value
mrbrs_obj_value(void* ptr);

struct RClass*
mrbrs_define_class(mrb_state* mrb, const char* name, struct RClass* superclass);

const char*
mrbrs_inspect(mrb_state* mrb, mrb_value obj, size_t* out_len);

struct RProc*
mrbrs_method_make_boxed_func(mrb_state* mrb, void* boxed_func);

void
mrbrs_define_method_proc(mrb_state* mrb, struct RClass* klass, const char* name, struct RProc* proc);

mrb_value
mrbrs_load_nstring(mrb_state* mrb, const char* s, size_t len);
