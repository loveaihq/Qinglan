//! Windows Imaging Component: COM declarations written by hand from `wincodec.h` (Windows SDK
//! 10.0.26100.0) and three safe functions on top: [`probe`] (the size from the header),
//! [`decode_scaled`] (first frame, straight to a display size, premultiplied BGRA) and
//! [`write_png`] (a PNG file from BGRA pixels, for `qinglan-cli render`).
//!
//! Vtable slots are counted from the method order in `wincodec.h`; `IUnknown` is slots 0 to 2:
//!
//! * `IWICImagingFactory`: 4 CreateDecoderFromStream, 8 CreateEncoder, 10 CreateFormatConverter, 11 CreateBitmapScaler.
//! * `IWICBitmapEncoder`: 3 Initialize, 10 CreateNewFrame, 11 Commit.
//! * `IWICBitmapFrameEncode`: 3 Initialize, 4 SetSize, 5 SetResolution, 6 SetPixelFormat, 10 WritePixels, 12 Commit.
//! * `IWICBitmapDecoder`: 12 GetFrameCount, 13 GetFrame.
//! * `IWICBitmapSource` (the frame, the converter and the scaler all derive from it): 3 GetSize,
//!   7 CopyPixels. `IWICFormatConverter` and `IWICBitmapScaler` add 8 Initialize (different arguments).
//!
//! The image bytes go in through `SHCreateMemStream`, which copies them into an `IStream`; a PNG goes out
//! through `SHCreateStreamOnFileEx` (an `IStream` on a new file).

use core::ffi::c_void;
use core::ptr::{NonNull, null, null_mut};

use super::{Guid, HRESULT};

type P = *mut c_void;

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoInitializeEx(reserved: *const c_void, co_init: u32) -> HRESULT;
    fn CoUninitialize();
    fn CoCreateInstance(clsid: *const Guid, outer: P, context: u32, iid: *const Guid, out: *mut P) -> HRESULT;
}

#[link(name = "shlwapi")]
unsafe extern "system" {
    fn SHCreateMemStream(init: *const u8, cb: u32) -> P;
    fn SHCreateStreamOnFileEx(file: *const u16, mode: u32, attributes: u32, create: i32, template: P, stream: *mut P) -> HRESULT;
}

const COINIT_MULTITHREADED: u32 = 0;
const RPC_E_CHANGED_MODE: HRESULT = 0x8001_0106_u32 as i32;
const CLSCTX_INPROC_SERVER: u32 = 1;
const E_FAIL: HRESULT = 0x8000_4005_u32 as i32;
const E_OUTOFMEMORY: HRESULT = 0x8007_000E_u32 as i32;
const WIC_DECODE_METADATA_CACHE_ON_DEMAND: u32 = 0;
const WIC_BITMAP_DITHER_NONE: u32 = 0;
const WIC_PALETTE_CUSTOM: u32 = 0;
const WIC_INTERPOLATION_FANT: u32 = 3;
const WIC_BITMAP_ENCODER_NO_CACHE: u32 = 2;
/// `STGM_CREATE | STGM_WRITE | STGM_SHARE_EXCLUSIVE`.
const STGM_CREATE_WRITE: u32 = 0x1000 | 0x1 | 0x10;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;

const CLSID_WIC_IMAGING_FACTORY: Guid = Guid { d1: 0xcacaf262, d2: 0x9370, d3: 0x4615, d4: [0xa1, 0x3b, 0x9f, 0x55, 0x39, 0xda, 0x4c, 0x0a] };
const IID_IWIC_IMAGING_FACTORY: Guid = Guid { d1: 0xec5ec8a9, d2: 0xc395, d3: 0x4314, d4: [0x9c, 0x77, 0x54, 0xd7, 0xa9, 0x35, 0xff, 0x70] };
const GUID_CONTAINER_FORMAT_PNG: Guid = Guid { d1: 0x1b7cfaf4, d2: 0x713f, d3: 0x473c, d4: [0xbb, 0xcd, 0x61, 0x37, 0x42, 0x5f, 0xae, 0xaf] };
const GUID_WIC_PIXEL_FORMAT_24BPP_BGR: Guid = Guid { d1: 0x6fddc324, d2: 0x4e03, d3: 0x4bfe, d4: [0xb1, 0x85, 0x3d, 0x77, 0x76, 0x8d, 0xc9, 0x0c] };
const GUID_WIC_PIXEL_FORMAT_32BPP_PBGRA: Guid = Guid { d1: 0x6fddc324, d2: 0x4e03, d3: 0x4bfe, d4: [0xb1, 0x85, 0x3d, 0x77, 0x76, 0x8d, 0xc9, 0x10] };

#[repr(C)]
struct Raw<V> {
    vtbl: *const V,
}

#[repr(C)]
struct UnknownVtbl {
    query_interface: unsafe extern "system" fn(P, *const Guid, *mut P) -> HRESULT,
    add_ref: unsafe extern "system" fn(P) -> u32,
    release: unsafe extern "system" fn(P) -> u32,
}

#[repr(C)]
struct FactoryVtbl {
    base: UnknownVtbl,
    _pad1: usize, // 3 CreateDecoderFromFilename
    create_decoder_from_stream: unsafe extern "system" fn(P, P, *const Guid, u32, *mut P) -> HRESULT, // 4
    _pad2: [usize; 3], // 5 FromFileHandle, 6 CreateComponentInfo, 7 CreateDecoder
    create_encoder: unsafe extern "system" fn(P, *const Guid, *const Guid, *mut P) -> HRESULT, // 8
    _pad3: usize, // 9 CreatePalette
    create_format_converter: unsafe extern "system" fn(P, *mut P) -> HRESULT, // 10
    create_bitmap_scaler: unsafe extern "system" fn(P, *mut P) -> HRESULT, // 11
}

#[repr(C)]
struct EncoderVtbl {
    base: UnknownVtbl,
    initialize: unsafe extern "system" fn(P, P, u32) -> HRESULT, // 3
    _pad1: [usize; 6], // 4 GetContainerFormat .. 9 SetPreview
    create_new_frame: unsafe extern "system" fn(P, *mut P, *mut P) -> HRESULT, // 10
    commit: unsafe extern "system" fn(P) -> HRESULT, // 11
}

#[repr(C)]
struct FrameEncodeVtbl {
    base: UnknownVtbl,
    initialize: unsafe extern "system" fn(P, P) -> HRESULT, // 3
    set_size: unsafe extern "system" fn(P, u32, u32) -> HRESULT, // 4
    _pad1: usize, // 5 SetResolution
    set_pixel_format: unsafe extern "system" fn(P, *mut Guid) -> HRESULT, // 6
    _pad2: [usize; 3], // 7 SetColorContexts, 8 SetPalette, 9 SetThumbnail
    write_pixels: unsafe extern "system" fn(P, u32, u32, u32, *const u8) -> HRESULT, // 10
    _pad3: usize, // 11 WriteSource
    commit: unsafe extern "system" fn(P) -> HRESULT, // 12
}

#[repr(C)]
struct DecoderVtbl {
    base: UnknownVtbl,
    _pad1: [usize; 9], // 3 QueryCapability .. 11 GetThumbnail
    get_frame_count: unsafe extern "system" fn(P, *mut u32) -> HRESULT, // 12
    get_frame: unsafe extern "system" fn(P, u32, *mut P) -> HRESULT,    // 13
}

#[repr(C)]
struct SourceVtbl {
    base: UnknownVtbl,
    get_size: unsafe extern "system" fn(P, *mut u32, *mut u32) -> HRESULT, // 3
    _pad1: [usize; 3], // 4 GetPixelFormat, 5 GetResolution, 6 CopyPalette
    copy_pixels: unsafe extern "system" fn(P, *const c_void, u32, u32, *mut u8) -> HRESULT, // 7
}

#[repr(C)]
struct ConverterVtbl {
    source: SourceVtbl,
    initialize: unsafe extern "system" fn(P, P, *const Guid, u32, P, f64, u32) -> HRESULT, // 8
}

#[repr(C)]
struct ScalerVtbl {
    source: SourceVtbl,
    initialize: unsafe extern "system" fn(P, P, u32, u32, u32) -> HRESULT, // 8
}

/// An owned COM interface pointer, released on drop.
struct ComPtr<V> {
    p: NonNull<Raw<V>>,
}

impl<V> ComPtr<V> {
    /// # Safety
    /// `p` is a valid interface pointer whose reference the caller hands over (or null, which fails).
    unsafe fn from_raw(p: P) -> Result<Self, HRESULT> {
        NonNull::new(p.cast::<Raw<V>>()).map(|p| ComPtr { p }).ok_or(E_FAIL)
    }
    fn raw(&self) -> P {
        self.p.as_ptr().cast()
    }
    fn vtbl(&self) -> &V {
        // SAFETY: every live COM object starts with a pointer to its vtable, which outlives the object;
        // `V` is declared to match the interface this pointer was obtained as.
        unsafe { &*(*self.p.as_ptr()).vtbl }
    }
}

impl<V> Drop for ComPtr<V> {
    fn drop(&mut self) {
        // SAFETY: by the COM ABI every vtable starts with IUnknown's three methods; we own one reference.
        unsafe {
            let vt = (*self.p.as_ptr()).vtbl.cast::<UnknownVtbl>();
            ((*vt).release)(self.raw());
        }
    }
}

fn check(hr: HRESULT) -> Result<(), HRESULT> {
    if hr < 0 { Err(hr) } else { Ok(()) }
}

/// COM is initialised once per thread (multithreaded apartment) and released when the thread ends. A
/// thread that already runs in a single-threaded apartment is left alone: WIC works in either.
struct ComGuard(bool);

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: balances the successful CoInitializeEx of this thread.
            unsafe { CoUninitialize() };
        }
    }
}

thread_local! {
    static COM: ComGuard = {
        // SAFETY: a null reserved pointer and a valid flag; per-thread initialisation.
        let hr = unsafe { CoInitializeEx(null(), COINIT_MULTITHREADED) };
        ComGuard(hr >= 0 && hr != RPC_E_CHANGED_MODE)
    };
}

fn factory() -> Result<ComPtr<FactoryVtbl>, HRESULT> {
    COM.with(|_| ());
    let mut p: P = null_mut();
    // SAFETY: valid GUID pointers; `p` receives an IWICImagingFactory pointer (we ask for that IID).
    check(unsafe { CoCreateInstance(&CLSID_WIC_IMAGING_FACTORY, null_mut(), CLSCTX_INPROC_SERVER, &IID_IWIC_IMAGING_FACTORY, &raw mut p) })?;
    // SAFETY: CoCreateInstance succeeded, so `p` is an owned interface pointer.
    unsafe { ComPtr::from_raw(p) }
}

/// The first frame of the image in `bytes`, and its size.
struct Opened {
    factory: ComPtr<FactoryVtbl>,
    frame: ComPtr<SourceVtbl>,
    width: u32,
    height: u32,
}

fn open(bytes: &[u8]) -> Result<Opened, HRESULT> {
    let len = u32::try_from(bytes.len()).map_err(|_| E_OUTOFMEMORY)?;
    let factory = factory()?;
    // SAFETY: `bytes` is valid for `len` bytes; SHCreateMemStream copies them.
    let stream_raw = unsafe { SHCreateMemStream(bytes.as_ptr(), len) };
    if stream_raw.is_null() {
        return Err(E_OUTOFMEMORY);
    }
    // The stream is an IUnknown-derived object; hold our reference in a ComPtr so it is released.
    // SAFETY: SHCreateMemStream returned a non-null owned IStream pointer.
    let stream: ComPtr<UnknownVtbl> = unsafe { ComPtr::from_raw(stream_raw)? };
    let mut dec_raw: P = null_mut();
    // SAFETY: the factory and stream are live; a null vendor GUID means "any decoder"; `dec_raw` is an out pointer.
    check(unsafe { (factory.vtbl().create_decoder_from_stream)(factory.raw(), stream.raw(), null(), WIC_DECODE_METADATA_CACHE_ON_DEMAND, &raw mut dec_raw) })?;
    // SAFETY: success: an owned IWICBitmapDecoder.
    let decoder: ComPtr<DecoderVtbl> = unsafe { ComPtr::from_raw(dec_raw)? };
    let mut frame_raw: P = null_mut();
    // SAFETY: the decoder is live; frame 0 is the first frame (GIF: first frame only).
    check(unsafe { (decoder.vtbl().get_frame)(decoder.raw(), 0, &raw mut frame_raw) })?;
    // SAFETY: success: an owned IWICBitmapFrameDecode, which begins with IWICBitmapSource's methods.
    let frame: ComPtr<SourceVtbl> = unsafe { ComPtr::from_raw(frame_raw)? };
    let (mut w, mut h) = (0u32, 0u32);
    // SAFETY: two live u32 out values.
    check(unsafe { (frame.vtbl().get_size)(frame.raw(), &raw mut w, &raw mut h) })?;
    // The decoder may go: the frame keeps what it needs alive.
    drop(decoder);
    drop(stream);
    Ok(Opened { factory, frame, width: w, height: h })
}

/// The pixel size claimed by the image header (the first frame). No pixels are decoded.
pub fn probe(bytes: &[u8]) -> Result<(u32, u32), HRESULT> {
    let o = open(bytes)?;
    Ok((o.width, o.height))
}

/// Decoded pixels: premultiplied BGRA, `width * 4` bytes per row.
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub natural_width: u32,
    pub natural_height: u32,
    pub bgra: Vec<u8>,
}

pub enum DecodeFail {
    /// The header claims more than the allowed number of pixels; nothing was decoded.
    TooLarge { width: u32, height: u32 },
    Hr(HRESULT),
}

/// Decode the first frame to `target_w` x `target_h` pixels. The conversion to premultiplied BGRA and
/// the scaling (Fant) are WIC pipeline stages that pull rows from the decoder on demand, so a big
/// image is never held at full size. Refuses images over `max_source_pixels`.
pub fn decode_scaled(bytes: &[u8], target_w: u32, target_h: u32, max_source_pixels: u64) -> Result<Pixels, DecodeFail> {
    let o = open(bytes).map_err(DecodeFail::Hr)?;
    if u64::from(o.width) * u64::from(o.height) > max_source_pixels {
        return Err(DecodeFail::TooLarge { width: o.width, height: o.height });
    }
    if target_w == 0 || target_h == 0 || o.width == 0 || o.height == 0 {
        return Err(DecodeFail::Hr(E_FAIL));
    }
    decode_inner(&o, target_w, target_h).map_err(DecodeFail::Hr)
}

fn decode_inner(o: &Opened, target_w: u32, target_h: u32) -> Result<Pixels, HRESULT> {
    let mut conv_raw: P = null_mut();
    // SAFETY: the factory is live; `conv_raw` is an out pointer.
    check(unsafe { (o.factory.vtbl().create_format_converter)(o.factory.raw(), &raw mut conv_raw) })?;
    // SAFETY: success: an owned IWICFormatConverter.
    let conv: ComPtr<ConverterVtbl> = unsafe { ComPtr::from_raw(conv_raw)? };
    // SAFETY: the frame is a live IWICBitmapSource; the format GUID outlives the call; no palette.
    check(unsafe { (conv.vtbl().initialize)(conv.raw(), o.frame.raw(), &GUID_WIC_PIXEL_FORMAT_32BPP_PBGRA, WIC_BITMAP_DITHER_NONE, null_mut(), 0.0, WIC_PALETTE_CUSTOM) })?;

    let stride = target_w.checked_mul(4).ok_or(E_OUTOFMEMORY)?;
    let size = stride.checked_mul(target_h).ok_or(E_OUTOFMEMORY)?;
    let mut buf = Vec::new();
    buf.try_reserve_exact(size as usize).map_err(|_| E_OUTOFMEMORY)?;
    buf.resize(size as usize, 0u8);

    if (target_w, target_h) == (o.width, o.height) {
        // SAFETY: `buf` holds `size` bytes, stride * height; a null rectangle means the whole image.
        check(unsafe { (conv.vtbl().source.copy_pixels)(conv.raw(), null(), stride, size, buf.as_mut_ptr()) })?;
    } else {
        let mut sc_raw: P = null_mut();
        // SAFETY: the factory is live; `sc_raw` is an out pointer.
        check(unsafe { (o.factory.vtbl().create_bitmap_scaler)(o.factory.raw(), &raw mut sc_raw) })?;
        // SAFETY: success: an owned IWICBitmapScaler.
        let sc: ComPtr<ScalerVtbl> = unsafe { ComPtr::from_raw(sc_raw)? };
        // SAFETY: the converter is a live IWICBitmapSource.
        check(unsafe { (sc.vtbl().initialize)(sc.raw(), conv.raw(), target_w, target_h, WIC_INTERPOLATION_FANT) })?;
        // SAFETY: as above for `buf`.
        check(unsafe { (sc.vtbl().source.copy_pixels)(sc.raw(), null(), stride, size, buf.as_mut_ptr()) })?;
    }
    Ok(Pixels { width: target_w, height: target_h, natural_width: o.width, natural_height: o.height, bgra: buf })
}

/// Rows converted and written per `WritePixels` call.
const PNG_ROWS_PER_WRITE: usize = 64;

/// Write a PNG file (24-bit, opaque) from premultiplied BGRA pixels, `width * 4` bytes per row. Pixels with
/// alpha under 255 are composed over white. `path` is a NUL-terminated UTF-16 file name; an existing file is replaced.
pub fn write_png(path: &[u16], width: u32, height: u32, bgra: &[u8]) -> Result<(), HRESULT> {
    let (w, h) = (width as usize, height as usize);
    let need = w.checked_mul(h).and_then(|n| n.checked_mul(4)).ok_or(E_OUTOFMEMORY)?;
    if w == 0 || h == 0 || path.last() != Some(&0) || bgra.len() < need {
        return Err(E_FAIL);
    }
    let factory = factory()?;
    let mut stream_raw: P = null_mut();
    // SAFETY: `path` is NUL-terminated UTF-16; a null template stream; `stream_raw` is an out pointer.
    check(unsafe { SHCreateStreamOnFileEx(path.as_ptr(), STGM_CREATE_WRITE, FILE_ATTRIBUTE_NORMAL, 1, null_mut(), &raw mut stream_raw) })?;
    // SAFETY: success: an owned IStream, released by the ComPtr (which also closes the file).
    let stream: ComPtr<UnknownVtbl> = unsafe { ComPtr::from_raw(stream_raw)? };
    let mut enc_raw: P = null_mut();
    // SAFETY: the factory is live; a null vendor GUID selects the system's PNG encoder; `enc_raw` is an out pointer.
    check(unsafe { (factory.vtbl().create_encoder)(factory.raw(), &GUID_CONTAINER_FORMAT_PNG, null(), &raw mut enc_raw) })?;
    // SAFETY: success: an owned IWICBitmapEncoder.
    let encoder: ComPtr<EncoderVtbl> = unsafe { ComPtr::from_raw(enc_raw)? };
    // SAFETY: the encoder and the stream are live; the stream is writable and seekable.
    check(unsafe { (encoder.vtbl().initialize)(encoder.raw(), stream.raw(), WIC_BITMAP_ENCODER_NO_CACHE) })?;
    let mut frame_raw: P = null_mut();
    // SAFETY: the encoder is live; the options bag is not wanted (a null out pointer is allowed); `frame_raw` is an out pointer.
    check(unsafe { (encoder.vtbl().create_new_frame)(encoder.raw(), &raw mut frame_raw, null_mut()) })?;
    // SAFETY: success: an owned IWICBitmapFrameEncode.
    let frame: ComPtr<FrameEncodeVtbl> = unsafe { ComPtr::from_raw(frame_raw)? };
    // SAFETY: the frame is live; null options mean the encoder's defaults.
    check(unsafe { (frame.vtbl().initialize)(frame.raw(), null_mut()) })?;
    // SAFETY: plain integers.
    check(unsafe { (frame.vtbl().set_size)(frame.raw(), width, height) })?;
    let mut format = GUID_WIC_PIXEL_FORMAT_24BPP_BGR;
    // SAFETY: `format` is a valid GUID that the encoder may change in place.
    check(unsafe { (frame.vtbl().set_pixel_format)(frame.raw(), &raw mut format) })?;
    if format != GUID_WIC_PIXEL_FORMAT_24BPP_BGR {
        return Err(E_FAIL);
    }
    let stride = w * 3;
    let mut rows = vec![0u8; stride * PNG_ROWS_PER_WRITE.min(h)];
    let mut y = 0;
    while y < h {
        let n = PNG_ROWS_PER_WRITE.min(h - y);
        let src = bgra.get(y * w * 4..(y + n) * w * 4).ok_or(E_FAIL)?;
        for (d, s) in rows.as_chunks_mut::<3>().0.iter_mut().zip(src.as_chunks::<4>().0) {
            let white = 255 - s[3];
            *d = [s[0].saturating_add(white), s[1].saturating_add(white), s[2].saturating_add(white)];
        }
        let bytes = stride * n;
        // SAFETY: `rows` holds at least `bytes` bytes: `n` rows of `stride` bytes each.
        check(unsafe { (frame.vtbl().write_pixels)(frame.raw(), n as u32, stride as u32, bytes as u32, rows.as_ptr()) })?;
        y += n;
    }
    // SAFETY: the frame has all its rows; Commit finishes it, then the encoder writes the file trailer.
    check(unsafe { (frame.vtbl().commit)(frame.raw()) })?;
    // SAFETY: as above, for the encoder.
    check(unsafe { (encoder.vtbl().commit)(encoder.raw()) })?;
    Ok(())
}
