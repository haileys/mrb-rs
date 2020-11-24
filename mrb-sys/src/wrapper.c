#include <string.h>

#include "wrapper.h"

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
mrbrs_define_class(mrb_state* mrb, const char* name, struct RClass* superclass, struct RObject** out_exc)
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
        mrb_gc_protect(mrb, mrb_obj_value(mrb->exc));
        *out_exc = mrb->exc;
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
