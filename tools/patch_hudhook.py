"""Makes our fixed copy of hudhook 0.9.3 (the overlay library, MIT) in third_party\\hudhook-0.9.3, which
genji\\Cargo.toml uses instead of the original ([patch.crates-io]). third_party\\ is never committed (AGENTS.md: third-party
code stays out of the repository), so this script makes it again from Rust's download cache: run by tools\\build.ps1
when the copy is missing.

The fix (step 10c, test of 2026-10-09): hudhook's `replace_texture` (Direct3D 12) created a new upload buffer on every
call and never released it (its own comment: "leaking the upload buffer ... is necessary"). Updating the arms' picture
every frame lost about 5 MB a frame: after about 90 s, "Not enough memory resources", and the game crashed when its
menu needed memory. It also put the texture back from the wrong state on every update after the first.
Fixed copy: one upload buffer per texture, made on the first update and reused; before each later update the texture
is moved back to the copy state.

Usage: python tools\\patch_hudhook.py   (does nothing if the fixed copy exists; --force to make it again)
"""
import glob
import os
import shutil
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEST = os.path.join(ROOT, 'third_party', 'hudhook-0.9.3')
MARK = os.path.join(DEST, 'GENJI_PATCHED')
# Raised whenever the fixes below change, so an older fixed copy is made again.
VERSION = 2

EDITS = [
    # The texture remembers its upload buffer and whether it has been filled before.
    ('''struct Texture {
    resource: ID3D12Resource,
    gpu_desc: D3D12_GPU_DESCRIPTOR_HANDLE,
    width: u32,
    height: u32,
}''', '''struct Texture {
    resource: ID3D12Resource,
    gpu_desc: D3D12_GPU_DESCRIPTOR_HANDLE,
    width: u32,
    height: u32,
    // GenjiBridge fix: reused upload buffer, and whether the texture was filled before (then it's in the
    // pixel-shader state and goes back to the copy state first).
    upload: Option<ID3D12Resource>,
    uploaded: bool,
}'''),
    ('''        self.textures.push(Texture { resource: texture.clone(), gpu_desc, width, height });''',
     '''        self.textures.push(Texture { resource: texture.clone(), gpu_desc, width, height, upload: None, uploaded: false });'''),
    ('''        let texture = &self.textures[texture_id.id()];
        if texture.width != width || texture.height != height {
            error!(
                "image size {width}x{height} do not match expected {}x{}",
                texture.width, texture.height
            );''', '''        let (tw, th) = (self.textures[texture_id.id()].width, self.textures[texture_id.id()].height);
        if tw != width || th != height {
            error!("image size {width}x{height} do not match expected {}x{}", tw, th);'''),
    ('''        let upload_buffer: ID3D12Resource = util::try_out_ptr(|v| unsafe {''',
     '''        // GenjiBridge fix: one upload buffer per texture, reused (was: a new one leaked on every call).
        let reuse = self.textures[texture_id.id()].upload.clone();
        let upload_buffer: ID3D12Resource = if let Some(b) = reuse { b } else {
        let b: ID3D12Resource = util::try_out_ptr(|v| unsafe {'''),
    ('''        })?;

        let mut upload_buffer_ptr = ptr::null_mut();''', '''        })?;
        self.textures[texture_id.id()].upload = Some(b.clone());
        b
        };

        let mut upload_buffer_ptr = ptr::null_mut();'''),
    ('''        let dst_location = D3D12_TEXTURE_COPY_LOCATION {''', '''        // GenjiBridge fix: a texture filled before is in the pixel-shader state: back to the copy state first.
        let texture = &self.textures[texture_id.id()];
        let to_copy: Vec<D3D12_RESOURCE_BARRIER> = if texture.uploaded {
            vec![util::create_barrier(
                &texture.resource,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                D3D12_RESOURCE_STATE_COPY_DEST,
            )]
        } else {
            Vec::new()
        };
        if !to_copy.is_empty() {
            self.command_list.ResourceBarrier(&to_copy);
        }

        let dst_location = D3D12_TEXTURE_COPY_LOCATION {'''),
    ('''        barriers.into_iter().for_each(util::drop_barrier);
''', '''        barriers.into_iter().for_each(util::drop_barrier);
        to_copy.into_iter().for_each(util::drop_barrier);
        self.textures[texture_id.id()].uploaded = true;
'''),
    # Second fix (test of 2026-10-09): copy only a rectangle of the picture (where the arms changed), not all of it:
    # the copy runs inside the game's frame (3-4 ms for 1720x720).
    ('''    unsafe fn upload_texture(
        &mut self,
        texture_id: TextureId,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<()> {''', '''    unsafe fn upload_texture(
        &mut self,
        texture_id: TextureId,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<()> {
        self.upload_texture_rect(texture_id, data, width, height, [0, 0, width, height])
    }

    // GenjiBridge fix: copies only the rectangle [x0, y0, x1, y1) of `data` (the whole picture, width x height).
    unsafe fn upload_texture_rect(
        &mut self,
        texture_id: TextureId,
        data: &[u8],
        width: u32,
        height: u32,
        rect: [u32; 4],
    ) -> Result<()> {'''),
    ('''        let upload_row_size = width * 4;
        let align = D3D12_TEXTURE_DATA_PITCH_ALIGNMENT;
        let upload_pitch = upload_row_size.div_ceil(align) * align; // 256 bytes
                                                                    // aligned
        let upload_size = height * upload_pitch;''', '''        let [x0, y0, x1, y1] = [rect[0].min(width), rect[1].min(height), rect[2].min(width), rect[3].min(height)];
        if x1 <= x0 || y1 <= y0 {
            return Ok(());
        }
        let (rw, rh) = (x1 - x0, y1 - y0);
        let upload_row_size = rw * 4;
        let align = D3D12_TEXTURE_DATA_PITCH_ALIGNMENT;
        let upload_pitch = upload_row_size.div_ceil(align) * align; // 256 bytes
                                                                    // aligned
        // The buffer is made once for the whole picture, so any rectangle fits.
        let upload_size = height * (width * 4).div_ceil(align) * align;'''),
    ('''        if upload_row_size == upload_pitch {
            ptr::copy_nonoverlapping(data.as_ptr(), upload_buffer_ptr as *mut u8, data.len());
        } else {
            for y in 0..height {
                let src = data.as_ptr().add((y * upload_row_size) as usize);
                let dst = (upload_buffer_ptr as *mut u8).add((y * upload_pitch) as usize);
                ptr::copy_nonoverlapping(src, dst, upload_row_size as usize);
            }
        }''', '''        for y in 0..rh {
            let src = data.as_ptr().add((((y0 + y) * width + x0) * 4) as usize);
            let dst = (upload_buffer_ptr as *mut u8).add((y * upload_pitch) as usize);
            ptr::copy_nonoverlapping(src, dst, upload_row_size as usize);
        }'''),
    ('''                        Width: width,
                        Height: height,''', '''                        Width: rw,
                        Height: rh,'''),
    ('''        self.command_list.CopyTextureRegion(&dst_location, 0, 0, 0, &src_location, None);''',
     '''        self.command_list.CopyTextureRegion(&dst_location, x0, y0, 0, &src_location, None);'''),
    ('''    ) -> Result<()> {
        unsafe { self.texture_heap.upload_texture(texture_id, data, width, height) }
    }
}''', '''    ) -> Result<()> {
        unsafe { self.texture_heap.upload_texture(texture_id, data, width, height) }
    }

    fn replace_texture_rect(
        &mut self,
        texture_id: TextureId,
        data: &[u8],
        width: u32,
        height: u32,
        rect: [u32; 4],
    ) -> Result<()> {
        unsafe { self.texture_heap.upload_texture_rect(texture_id, data, width, height, rect) }
    }
}'''),
]

# The trait gets the new method (other renderers copy the whole picture).
LIB_EDITS = [
    ('''    fn replace_texture(
        &mut self,
        texture_id: TextureId,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), Error>;
}''', '''    fn replace_texture(
        &mut self,
        texture_id: TextureId,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), Error>;

    /// GenjiBridge addition: like `replace_texture`, but only the rectangle [x0, y0, x1, y1) of `data` (the whole
    /// picture, width x height) is copied. Renderers without it copy everything.
    fn replace_texture_rect(
        &mut self,
        texture_id: TextureId,
        data: &[u8],
        width: u32,
        height: u32,
        rect: [u32; 4],
    ) -> Result<(), Error> {
        let _ = rect;
        self.replace_texture(texture_id, data, width, height)
    }
}'''),
]


def main():
    if os.path.isfile(MARK) and f'version {VERSION}' in open(MARK).read() and '--force' not in sys.argv:
        print(f'third_party/hudhook-0.9.3: fixed copy (version {VERSION}) already there')
        return
    home = os.path.expanduser('~')
    found = glob.glob(os.path.join(home, '.cargo', 'registry', 'src', '*', 'hudhook-0.9.3'))
    if not found:
        sys.exit('hudhook 0.9.3 is not in the cargo cache: run "cargo fetch" in genji\\ first')
    if os.path.isdir(DEST):
        shutil.rmtree(DEST)
    shutil.copytree(found[0], DEST)
    path = os.path.join(DEST, 'src', 'renderer', 'backend', 'dx12.rs')
    src = open(path, encoding='utf-8').read()
    for old, new in EDITS:
        if src.count(old) != 1:
            sys.exit(f'patch_hudhook: expected text found {src.count(old)} times in dx12.rs:\n{old[:200]}')
        src = src.replace(old, new)
    open(path, 'w', encoding='utf-8').write(src)
    lib = os.path.join(DEST, 'src', 'lib.rs')
    src = open(lib, encoding='utf-8').read()
    for old, new in LIB_EDITS:
        if src.count(old) != 1:
            sys.exit(f'patch_hudhook: expected text found {src.count(old)} times in lib.rs:\n{old[:200]}')
        src = src.replace(old, new)
    open(lib, 'w', encoding='utf-8').write(src)
    open(MARK, 'w').write(f'patched by tools/patch_hudhook.py, version {VERSION}\n')
    print('third_party/hudhook-0.9.3: copied from', found[0], 'and fixed')


if __name__ == '__main__':
    main()
