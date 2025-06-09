package ffi

// #cgo CFLAGS: -I.
// #include "dash_sdk_ffi.h"
// #include <stdlib.h>
// #include <string.h>
import "C"
import (
	"unsafe"
)

// GoStringToC converts a Go string to a C string (caller must free)
func GoStringToC(s string) *C.char {
	return C.CString(s)
}

// CStringToGo converts a C string to a Go string
func CStringToGo(s *C.char) string {
	if s == nil {
		return ""
	}
	return C.GoString(s)
}

// CStringToGoAndFree converts a C string to Go string and frees the C string
func CStringToGoAndFree(s *C.char) string {
	if s == nil {
		return ""
	}
	result := C.GoString(s)
	FreeString(s)
	return result
}

// GoBytesToC converts Go bytes to C array (caller must free)
func GoBytesToC(data []byte) (unsafe.Pointer, C.size_t) {
	if len(data) == 0 {
		return nil, 0
	}
	
	cData := C.malloc(C.size_t(len(data)))
	C.memcpy(cData, unsafe.Pointer(&data[0]), C.size_t(len(data)))
	return cData, C.size_t(len(data))
}

// CBytesToGo converts C bytes to Go slice
func CBytesToGo(data unsafe.Pointer, size C.size_t) []byte {
	if data == nil || size == 0 {
		return nil
	}
	
	return C.GoBytes(data, C.int(size))
}

// CBytesToGoAndFree converts C bytes to Go slice and frees the C memory
func CBytesToGoAndFree(data unsafe.Pointer, size C.size_t) []byte {
	if data == nil || size == 0 {
		return nil
	}
	
	result := C.GoBytes(data, C.int(size))
	C.free(data)
	return result
}

// GoBytes32ToC converts a 32-byte Go array to C array
func GoBytes32ToC(data [32]byte) *[32]C.uint8_t {
	var cData [32]C.uint8_t
	for i := 0; i < 32; i++ {
		cData[i] = C.uint8_t(data[i])
	}
	return &cData
}

// CBytes32ToGo converts a 32-byte C array to Go array
func CBytes32ToGo(data *[32]C.uint8_t) [32]byte {
	var goData [32]byte
	if data != nil {
		for i := 0; i < 32; i++ {
			goData[i] = byte(data[i])
		}
	}
	return goData
}

// GoBytes20ToC converts a 20-byte Go array to C array
func GoBytes20ToC(data [20]byte) *[20]C.uint8_t {
	var cData [20]C.uint8_t
	for i := 0; i < 20; i++ {
		cData[i] = C.uint8_t(data[i])
	}
	return &cData
}

// CBytes20ToGo converts a 20-byte C array to Go array
func CBytes20ToGo(data *[20]C.uint8_t) [20]byte {
	var goData [20]byte
	if data != nil {
		for i := 0; i < 20; i++ {
			goData[i] = byte(data[i])
		}
	}
	return goData
}

// AllocateC allocates C memory
func AllocateC(size int) unsafe.Pointer {
	return C.malloc(C.size_t(size))
}

// FreeC frees C memory
func FreeC(ptr unsafe.Pointer) {
	if ptr != nil {
		C.free(ptr)
	}
}

// CopyToC copies Go data to C memory
func CopyToC(dst unsafe.Pointer, src []byte) {
	if dst != nil && len(src) > 0 {
		C.memcpy(dst, unsafe.Pointer(&src[0]), C.size_t(len(src)))
	}
}