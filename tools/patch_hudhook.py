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
]


def main():
    if os.path.isfile(MARK) and '--force' not in sys.argv:
        print('third_party/hudhook-0.9.3: fixed copy already there')
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
    open(MARK, 'w').write('patched by tools/patch_hudhook.py\n')
    print('third_party/hudhook-0.9.3: copied from', found[0], 'and fixed')


if __name__ == '__main__':
    main()
