#include <stdlib.h>
#include <string.h>

#include "wrapper.h"

mrb_state*
mrbrs_open_core()
{
    // allocate userdata struct for mrbrs

    mrbrs_ud* ud = calloc(1, sizeof(mrbrs_ud));

    if (!ud) {
        return NULL;
    }

    // open mruby

    mrb_state* mrb = mrb_open_core(mrb_default_allocf, NULL);

    if (!mrb) {
        // free ud if we can't open mruby
        free(ud);
        return NULL;
    }

    mrb->ud = ud;

    // initialize panic carrier exception in try block
    // this is an exception that needs to be uncatchable by Ruby, so we clone
    // BasicObject to create a parallel class hierarchy root and create an
    // instance of said class.

    struct mrb_jmpbuf jmp;
    MRB_TRY(&jmp) {
        int ai = mrb_gc_arena_save(mrb);
        mrb->jmp = &jmp;

        struct RClass* basic_object = mrb_class_get(mrb, "BasicObject");

        mrb_value carrier_obj = mrb_obj_dup(mrb, mrb_obj_value(basic_object));
        mrb_gc_protect(mrb, carrier_obj);

        struct RClass* carrier = mrb_class_ptr(carrier_obj);
        mrb_value ex_panic_obj = mrb_obj_new(mrb, carrier, 0, NULL);
        mrb_gc_protect(mrb, ex_panic_obj);

        ud->panic_carrier = mrb_obj_ptr(ex_panic_obj);

        mrb_gc_arena_restore(mrb, ai);
        mrb->jmp = NULL;
    } MRB_CATCH(&jmp) {
        // something went wrong
        mrbrs_close(mrb);
        mrb = NULL;
    } MRB_END_EXC(&jmp);

    return mrb;
}

void
mrbrs_close(mrb_state* mrb)
{
    free(mrb->ud);
    mrb_close(mrb);
}

int
mrbrs_gc_arena_save(mrb_state *mrb)
{
    return mrb_gc_arena_save(mrb);
}

void
mrbrs_gc_arena_restore(mrb_state *mrb, int idx)
{
    mrb_gc_arena_restore(mrb, idx);
}

mrb_value
mrbrs_obj_value(void* ptr)
{
    return mrb_obj_value(ptr);
}

struct RClass*
mrbrs_define_class(mrb_state* mrb, const char* name, struct RClass* superclass)
{
    struct mrb_jmpbuf* prev_jmp = mrb->jmp;
    struct mrb_jmpbuf jmp;
    struct RClass* result = NULL;

    MRB_TRY(&jmp) {
        mrb->jmp = &jmp;
        result = mrb_define_class(mrb, name, superclass);
        mrb_gc_protect(mrb, mrb_obj_value(result));
        mrb->jmp = prev_jmp;
    } MRB_CATCH(&jmp) {
        mrb->jmp = prev_jmp;
    } MRB_END_EXC(&jmp);

    return result;
}

const char*
mrbrs_inspect(mrb_state* mrb, mrb_value obj, size_t* out_len)
{
    struct mrb_jmpbuf* prev_jmp = mrb->jmp;
    struct mrb_jmpbuf jmp, jmp2;
    const char* result = NULL;

    MRB_TRY(&jmp) {
        mrb->jmp = &jmp;

        mrb_value inspect = mrb_inspect(mrb, obj);
        mrb_gc_protect(mrb, inspect);
        mrb_obj_freeze(mrb, inspect);

        result = RSTRING_PTR(inspect);
        *out_len = RSTRING_LEN(inspect);

        mrb->jmp = prev_jmp;
    } MRB_CATCH(&jmp) {
        // exception in mrb_inspect, let's try mrb_any_to_s instead
        MRB_TRY(&jmp2) {
            mrb->jmp = &jmp2;

            mrb_value inspect = mrb_any_to_s(mrb, obj);
            mrb_gc_protect(mrb, inspect);
            mrb_obj_freeze(mrb, inspect);

            result = RSTRING_PTR(inspect);
            *out_len = RSTRING_LEN(inspect);

            mrb->jmp = prev_jmp;
        } MRB_CATCH(&jmp2) {
            // exception in mrb_any_to_s! things must be really broken

            result = "#<" "???" ">"; // use multiple string lits to break up trigraph
            *out_len = strlen(result);

            mrb->jmp = prev_jmp;
        } MRB_END_EXC(&jmp2);
    } MRB_END_EXC(&jmp2);

    return result;
}


void mrbrs_method_free_boxed_func(mrb_state*, void*);
void mrbrs_method_dispatch_boxed_func(mrb_state*, mrb_value, void*, mrb_value*);

static mrb_data_type
boxed_func_data_type = {
    .struct_name = "mrbrs::method::BoxedFunc",
    .dfree = mrbrs_method_free_boxed_func,
};

static mrb_value
boxed_func_dispatch(mrb_state* mrb, mrb_value self)
{
    mrb_value data_obj = mrb_proc_cfunc_env_get(mrb, 0);
    void* data = mrb_data_get_ptr(mrb, data_obj, &boxed_func_data_type);

    mrb_value retn = mrb_undef_value();
    mrbrs_method_dispatch_boxed_func(mrb, self, data, &retn);

    if (mrb->exc) {
        MRB_THROW(mrb->jmp);
    }

    return retn;
}

struct RProc*
mrbrs_method_make_boxed_func(mrb_state* mrb, void* boxed_func)
{
    struct mrb_jmpbuf* prev_jmp = mrb->jmp;
    struct mrb_jmpbuf jmp;
    struct RProc* result = NULL;

    MRB_TRY(&jmp) {
        mrb->jmp = &jmp;

        struct mrb_value data =
            mrb_obj_value(
                mrb_data_object_alloc(
                    mrb,
                    NULL,
                    boxed_func,
                    &boxed_func_data_type));

        mrb_gc_protect(mrb, data);

        result = mrb_proc_new_cfunc_with_env(
            mrb,
            boxed_func_dispatch,
            1,
            &data);

        mrb_gc_protect(mrb, mrb_obj_value(result));

        mrb->jmp = prev_jmp;
    } MRB_CATCH(&jmp) {
        mrb->jmp = prev_jmp;
    } MRB_END_EXC(&jmp);

    return result;
}

void
mrbrs_define_method_proc(mrb_state* mrb, struct RClass* klass, const char* name, struct RProc* proc)
{
    struct mrb_jmpbuf* prev_jmp = mrb->jmp;
    struct mrb_jmpbuf jmp;

    MRB_TRY(&jmp) {
        mrb->jmp = &jmp;

        mrb_sym mid = mrb_intern_cstr(mrb, name);

        mrb_method_t m;
        MRB_METHOD_FROM_PROC(m, proc);
        mrb_define_method_raw(mrb, klass, mid, m);

        mrb->jmp = prev_jmp;
    } MRB_CATCH(&jmp) {
        mrb->jmp = prev_jmp;
    } MRB_END_EXC(&jmp);
}

mrb_value
mrbrs_load_nstring(mrb_state* mrb, const char* s, size_t len)
{
    struct mrb_jmpbuf* prev_jmp = mrb->jmp;
    struct mrb_jmpbuf jmp;
    mrb_value result = mrb_nil_value();

    int ai = mrb_gc_arena_save(mrb);

    MRB_TRY(&jmp) {
        mrb->jmp = &jmp;

        result = mrb_load_nstring(mrb, s, len);
        mrb_gc_arena_restore(mrb, ai);
        mrb_gc_protect(mrb, result);

        mrb->jmp = prev_jmp;
    } MRB_CATCH(&jmp) {
        mrb_gc_arena_restore(mrb, ai);
        mrb->jmp = prev_jmp;
    } MRB_END_EXC(&jmp);

    return result;
}
