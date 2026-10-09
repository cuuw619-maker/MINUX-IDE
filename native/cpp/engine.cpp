#include <cstdint>

// Stable C ABI boundary used by the Rust host.
extern "C" std::uint32_t minux_engine_version() noexcept {
    return 1;
}
