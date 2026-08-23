#include <dash/platform/ffi.h>

#include <cstdint>

int main()
{
    try {
        platform_ffi::set_context("test", std::uint32_t{0});
    } catch (const rust::Error&) {
        return 1;
    }
    return 0;
}
