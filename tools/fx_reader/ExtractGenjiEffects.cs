// GenjiBridge's effect reader (docs/DESIGN.md 5l, part 6): one extra DataTool mode, `extract-genji-effects`.
// tools/extract_effects.ps1 copies this file into DataTool's own source, builds it and runs it on your offline
// copy of Overwatch. Read-only on the game files.
//
// Why: DataTool no longer writes its effect files (.oweffect), and its effect parser skips particles and ribbons
// (Overwatch's trails). This mode walks every effect Genji's entities use, chunk by chunk, and writes:
//   effects.txt  every effect: the animations and entities that start it, and each part in order with its timing,
//                what it is, its material's textures and constants, the particle system's own chunks, and the raw
//                numbers of everything TankLib doesn't decode;
//   index.txt    one line per effect: animations, sounds, part counts;
//   textures/    each texture those effects use, once;
//   raw/         each chunk's bytes.
// The hero is GENJI_FX_HERO (default Genji). Default skin (no skin replacements).

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using DataTool.Flag;
using DataTool.Helper;
using TankLib;
using TankLib.Chunks;
using TankLib.STU.Types;
using Logger = TankLib.Helpers.Logger;

namespace DataTool.ToolLogic.Extract;

[Tool("extract-genji-effects", Description = "GenjiBridge: every effect of one hero, with parts, timings, materials, textures", CustomFlags = typeof(ExtractFlags))]
public class ExtractGenjiEffects : ITool {
    private record Chunk(string Tag, int Depth, ushort Version, byte[] Data);

    // GUID types worth naming when found inside raw chunk data.
    private static readonly HashSet<ushort> GuidTypes = [0x4, 0x6, 0x8, 0xC, 0xD, 0x1A, 0x2C, 0x3, 0x8F, 0xB3, 0xF1, 0x4A, 0x8E];

    private FindLogic.Combo.ComboInfo _info;
    private SaveLogic.Combo.SaveContext _save;
    private ExtractFlags _flags;
    private string _out;
    private readonly HashSet<ulong> _savedTextures = [];

    public void Parse(ICLIFlags toolFlags) {
        _flags = (ExtractFlags) toolFlags;
        _flags.EnsureOutputDirectory();
        _out = _flags.OutputPath;
        var heroName = Environment.GetEnvironmentVariable("GENJI_FX_HERO") ?? "Genji";

        var hero = Helpers.GetHeroes().Values.FirstOrDefault(h => string.Equals(h.Name, heroName, StringComparison.OrdinalIgnoreCase));
        if (hero?.STU == null) {
            Logger.Error("GenjiFX", $"hero {heroName} not found");
            return;
        }

        _info = new FindLogic.Combo.ComboInfo();
        SaveLogic.Unlock.SkinTheme.FindEntities(_info, default, hero.STU);
        _save = new SaveLogic.Combo.SaveContext(_info);
        Logger.Info("GenjiFX", $"{hero.Name}: {_info.m_entities.Count} entities, {_info.m_animations.Count} animations, " +
                               $"{_info.m_animationEffects.Count} animation effects, {_info.m_effects.Count} other effects");

        // Who starts each effect: animations (by their effect) and entities.
        var byAnimation = new Dictionary<ulong, List<ulong>>();
        foreach (var anim in _info.m_animations.Values) {
            if (anim.m_effect == 0) continue;
            if (!byAnimation.TryGetValue(anim.m_effect, out var list)) byAnimation[anim.m_effect] = list = [];
            list.Add(anim.m_GUID);
        }
        var byEntity = new Dictionary<ulong, List<string>>();
        foreach (var entity in _info.m_entities.Values) {
            foreach (var fx in entity.m_effects) {
                if (!byEntity.TryGetValue(fx, out var list)) byEntity[fx] = list = [];
                list.Add($"{G(entity.m_GUID)} {entity.GetName()}");
            }
        }

        var report = new StringBuilder();
        var index = new StringBuilder();
        index.AppendLine("effect | started by animations | entities | sounds (02C) | parts");
        var effects = _info.m_animationEffects.Keys.Concat(_info.m_effects.Keys).Distinct().OrderBy(teResourceGUID.Index).ToList();
        foreach (var fx in effects) {
            try {
                Effect(fx, byAnimation.GetValueOrDefault(fx), byEntity.GetValueOrDefault(fx), report, index);
            } catch (Exception e) {
                report.AppendLine($"  !! failed: {e.Message}");
                Logger.Warn("GenjiFX", $"{G(fx)}: {e.Message}");
            }
        }

        File.WriteAllText(Path.Combine(_out, "effects.txt"), report.ToString());
        File.WriteAllText(Path.Combine(_out, "index.txt"), index.ToString());
        Abilities(effects);
        Logger.Success("GenjiFX", $"{effects.Count} effects, {_savedTextures.Count} textures, written to {_out}");
    }

    /// Which ability starts which effect. Overwatch starts ability effects from scripts (statescript graphs, 0x1B);
    /// a script's bytes name the abilities (loadouts, 0x9E) it belongs to. Each graph is walked on its own (with the
    /// graphs it pulls in), and each effect is put under the smallest graph that names an ability and reaches it.
    /// Writes abilities.txt (ability -> effects) and graphs.txt (every graph: abilities named, effects reached).
    private void Abilities(List<ulong> effects) {
        var loadouts = Helpers.GetLoadouts();
        var graphs = new List<(ulong Graph, List<string> Names, HashSet<ulong> Effects)>();
        foreach (var graph in _info.m_doneScripts.OrderBy(teResourceGUID.Index)) {
            var sub = new FindLogic.Combo.ComboInfo();
            try {
                FindLogic.Combo.Find(sub, graph);
            } catch (Exception e) {
                Logger.Warn("GenjiFX", $"graph {G(graph)}: {e.Message}");
                continue;
            }
            var reached = sub.m_effects.Keys.Concat(sub.m_animationEffects.Keys).ToHashSet();
            var names = new HashSet<string>();
            var bytes = Read(graph) ?? [];
            for (int o = 0; o + 8 <= bytes.Length; o++) {
                ulong v = BitConverter.ToUInt64(bytes, o);
                if (teResourceGUID.Type(v) != 0x9E || (v >> 60) != 0) continue;
                if (loadouts.TryGetValue((teResourceGUID) v, out var l) && l.Name != null) names.Add(l.Name);
            }
            graphs.Add((graph, names.OrderBy(n => n).ToList(), reached));
        }

        var g = new StringBuilder();
        foreach (var (graph, names, reached) in graphs) {
            g.AppendLine($"{G(graph)} | abilities: {(names.Count == 0 ? "-" : string.Join(", ", names))} | {reached.Count} effects: {string.Join(" ", reached.OrderBy(teResourceGUID.Index).Select(G))}");
        }
        File.WriteAllText(Path.Combine(_out, "graphs.txt"), g.ToString());

        var byAbility = new SortedDictionary<string, List<string>>();
        foreach (var fx in effects) {
            var best = graphs.Where(x => x.Names.Count > 0 && x.Effects.Contains(fx)).OrderBy(x => x.Effects.Count).FirstOrDefault();
            var key = best.Names == null ? "(no ability found)" : string.Join(" + ", best.Names);
            if (!byAbility.TryGetValue(key, out var list)) byAbility[key] = list = [];
            list.Add(best.Names == null ? G(fx) : $"{G(fx)} (graph {G(best.Graph)}, {best.Effects.Count} effects)");
        }
        var a = new StringBuilder();
        foreach (var (ability, list) in byAbility) {
            a.AppendLine($"== {ability}");
            foreach (var line in list) a.AppendLine($"  {line}");
        }
        File.WriteAllText(Path.Combine(_out, "abilities.txt"), a.ToString());
        Logger.Info("GenjiFX", $"{graphs.Count} scripts, {graphs.Count(x => x.Names.Count > 0)} naming an ability");
    }

    private static string G(ulong guid) => guid == 0 ? "-" : $"{teResourceGUID.LongKey(guid):X12}.{teResourceGUID.Type(guid):X3}";

    private void Effect(ulong fx, List<ulong> animations, List<string> entities, StringBuilder report, StringBuilder index) {
        report.AppendLine($"==== EFFECT {G(fx)}");
        if (animations != null) report.AppendLine($"  started by animations: {string.Join(", ", animations.Select(G))}");
        if (entities != null) report.AppendLine($"  on entities: {string.Join("; ", entities)}");

        var bytes = Read(fx);
        if (bytes == null) {
            report.AppendLine("  (file not readable)");
            return;
        }
        var chunks = Walk(bytes, out string fileTag);
        report.AppendLine($"  file tag {fileTag}, {bytes.Length} bytes, {chunks.Count} chunks");

        var counts = new SortedDictionary<string, int>();
        var sounds = new List<string>();
        var rawDir = Path.Combine(_out, "raw", G(fx));
        Directory.CreateDirectory(rawDir);
        for (int i = 0; i < chunks.Count; i++) {
            var c = chunks[i];
            counts[c.Tag] = counts.GetValueOrDefault(c.Tag) + 1;
            File.WriteAllBytes(Path.Combine(rawDir, $"{i:D3}_{Safe(c.Tag)}.bin"), c.Data);
            var pad = new string(' ', 2 + 2 * c.Depth);
            report.AppendLine($"{pad}[{i}] {c.Tag} v{c.Version}, {c.Data.Length} bytes");
            var decoded = Decode(c, fileTag, pad + "    ", report);
            if (decoded is teEffectComponentSound s) sounds.Add(G(s.Header.Sound));
            if (decoded == null || decoded is teDataChunk_Dummy) Raw(c.Data, pad + "    ", report);
        }
        report.AppendLine();

        index.AppendLine($"{G(fx)} | {(animations == null ? "-" : string.Join(" ", animations.Select(G)))} | " +
                         $"{(entities == null ? "-" : string.Join("; ", entities))} | {(sounds.Count == 0 ? "-" : string.Join(" ", sounds))} | " +
                         string.Join(" ", counts.Select(kv => $"{kv.Key}x{kv.Value}")));
    }

    private static string Safe(string tag) => new(tag.Select(ch => char.IsLetterOrDigit(ch) ? ch : '_').ToArray());

    private static byte[] Read(ulong guid) {
        using var stream = IO.OpenFile(guid);
        if (stream == null) return null;
        using var ms = new MemoryStream();
        stream.CopyTo(ms);
        return ms.ToArray();
    }

    /// The chunks of a chunked file, in file order with their nesting depth, the way teChunkedData walks them, but
    /// keeping every chunk's bytes (TankLib drops the ones it doesn't know).
    private static List<Chunk> Walk(byte[] bytes, out string fileTag) {
        var list = new List<Chunk>();
        fileTag = "?";
        using var ms = new MemoryStream(bytes);
        using var reader = new BinaryReader(ms);
        if (bytes.Length < 16) return list;
        var header = reader.Read<teChunkDataHeader>();
        if (header.Magic != teChunkedData.Magic) {
            fileTag = $"not chunked (magic {header.Magic:X8})";
            return list;
        }
        fileTag = header.StringIdentifier;
        try {
            while (ms.Position < header.Size && ms.Position < bytes.Length) {
                long before = ms.Position;
                Level(reader, list, 0);
                if (ms.Position <= before) break;
            }
        } catch (Exception e) {
            list.Add(new Chunk($"!! walk stopped: {e.Message}", 0, 0, []));
        }
        return list;
    }

    private static void Level(BinaryReader reader, List<Chunk> list, int depth) {
        var s = reader.BaseStream;
        var entry = reader.Read<teChunkDataEntry>();
        var rootSize = entry.SerializedSize;
        var start = s.Position;
        var startAbs = start;
        while (entry.SerializationTag == 0xC65C) {
            if (entry.ChunkSize < 0 || start + entry.ChunkSize > s.Length) throw new InvalidDataException($"chunk {entry.StringIdentifier} runs past the end");
            list.Add(new Chunk(entry.StringIdentifier, depth, entry.ChunkVersion, reader.ReadBytes(entry.ChunkSize)));
            s.Position = start + entry.ChunkSize;
            if (entry.SerializedSize > entry.ChunkSize) Level(reader, list, depth + 1);
            if (s.Position - startAbs >= rootSize) break;
            entry = reader.Read<teChunkDataEntry>();
            start = s.Position;
        }
        s.Position = start + entry.SerializedSize;
    }

    /// The chunk as TankLib reads it, with what it points at written out; null if TankLib doesn't know it.
    private IChunk Decode(Chunk c, string fileTag, string pad, StringBuilder report) {
        IChunk chunk;
        try {
            chunk = teChunkedData.Manager.CreateChunkInstance(c.Tag, fileTag);
            if (chunk == null) return null;
            chunk.Parse(new MemoryStream(c.Data));
        } catch (Exception e) {
            report.AppendLine($"{pad}(TankLib couldn't read it: {e.Message})");
            return null;
        }
        switch (chunk) {
            case teEffectChunkComponent cmp:
                report.AppendLine($"{pad}timing: start {cmp.StartTime:0.###} s, length {cmp.Duration:0.###} s, hardpoint {G(cmp.Header.Hardpoint)}, index {cmp.Header.Index}, floats {cmp.Header.FloatOneA:0.###} {cmp.Header.FloatOneB:0.###}");
                break;
            case teEffectComponentParticle p:
                report.AppendLine($"{pad}PARTICLE SYSTEM {G(p.Header.Model)} (unknowns {p.Header.Unknown1:X} {p.Header.Unknown2:X} {p.Header.Unknown3:X} {p.Header.Unknown4:X})");
                System(p.Header.Model, pad + "  ", report);
                break;
            case teEffectComponentRibbonRenderer rr:
                report.AppendLine($"{pad}RIBBON {G(rr.Header.ModelGUID)}");
                System(rr.Header.ModelGUID, pad + "  ", report);
                break;
            case teEffectChunkShaderSetup ss:
                report.AppendLine($"{pad}MATERIAL {G(ss.Header.Material)}, data {G(ss.Header.MaterialData)} (unknown {ss.Header.Unknown:X})");
                Material(ss.Header.Material, ss.Header.MaterialData, pad + "  ", report);
                break;
            case teEffectComponentModel m:
                report.AppendLine($"{pad}MESH {G(m.Header.Model)}, look {G(m.Header.ModelLook)}, animation {G(m.Header.Animation)}");
                Look(m.Header.ModelLook, pad + "  ", report);
                break;
            case teEffectComponentEffect e:
                report.AppendLine($"{pad}SUB-EFFECT {G(e.Header.Effect)}");
                break;
            case teEffectComponentSound snd:
                report.AppendLine($"{pad}SOUND {G(snd.Header.Sound)}");
                break;
            case teEffectComponentEntity ent:
                report.AppendLine($"{pad}ENTITY {G(ent.Header.Entity)}, identifier {G(ent.Header.Identifier)}");
                break;
            case teEffectComponentEntityControl ec:
                report.AppendLine($"{pad}ENTITY CONTROL {ec.Header.Action}, animation {G(ec.Header.Animation)}, identifier {G(ec.Header.Identifier)}");
                break;
            case teEffectComponentVoiceStimulus vs:
                report.AppendLine($"{pad}VOICE {G(vs.Header.VoiceStimulus)}");
                break;
        }
        return chunk;
    }

    /// A particle system or ribbon "model": its own chunks (the emitter's settings).
    private void System(ulong guid, string pad, StringBuilder report) {
        var bytes = guid == 0 ? null : Read(guid);
        if (bytes == null) {
            report.AppendLine($"{pad}(not readable)");
            return;
        }
        var chunks = Walk(bytes, out string tag);
        report.AppendLine($"{pad}{G(guid)}: file tag {tag}, {bytes.Length} bytes, {chunks.Count} chunks");
        var dir = Path.Combine(_out, "raw", G(guid));
        Directory.CreateDirectory(dir);
        for (int i = 0; i < chunks.Count; i++) {
            var c = chunks[i];
            File.WriteAllBytes(Path.Combine(dir, $"{i:D3}_{Safe(c.Tag)}.bin"), c.Data);
            var p = pad + new string(' ', 2 * c.Depth);
            report.AppendLine($"{p}<{i}> {c.Tag} v{c.Version}, {c.Data.Length} bytes");
            Raw(c.Data, p + "    ", report);
        }
    }

    private void Look(ulong look, string pad, StringBuilder report) {
        if (look == 0 || !_info.m_modelLooks.TryGetValue(look, out var info)) return;
        foreach (var mat in info.m_materials) {
            report.AppendLine($"{pad}look material {G(mat.m_guid)}");
            Material(mat.m_guid, 0, pad + "  ", report);
        }
    }

    /// A material's textures (saved to textures/) and constants (as numbers).
    private void Material(ulong material, ulong materialData, string pad, StringBuilder report) {
        if (material != 0 && materialData == 0 && _info.m_materials.TryGetValue(material, out var m)) {
            materialData = m.m_materialDataGUID;
            report.AppendLine($"{pad}its data {G(materialData)}");
        }
        if (materialData == 0) return;
        if (!_info.m_materialData.ContainsKey(materialData)) FindLogic.Combo.Find(_info, materialData);
        if (!_info.m_materialData.TryGetValue(materialData, out var data)) {
            report.AppendLine($"{pad}(material data not found)");
            return;
        }
        if (data.m_textureMap != null) {
            foreach (var (hash, tex) in data.m_textureMap) {
                string saved = SaveTexture(tex);
                report.AppendLine($"{pad}texture {hash:X8}: {G(tex)} {saved}");
            }
        }
        if (data.m_staticInputMap != null) {
            foreach (var (hash, value) in data.m_staticInputMap) {
                report.AppendLine($"{pad}constant {hash:X8} ({value.Length} bytes): {Floats(value)}");
            }
        }
    }

    private string SaveTexture(ulong tex) {
        if (!_info.m_textures.ContainsKey(tex)) FindLogic.Combo.Find(_info, tex);
        if (!_info.m_textures.ContainsKey(tex)) return "(not found)";
        if (_savedTextures.Add(tex)) {
            try {
                SaveLogic.Combo.SaveTexture(_flags, Path.Combine(_out, "textures"), _save, tex);
            } catch (Exception e) {
                return $"(not saved: {e.Message})";
            }
        }
        return $"-> textures/{_info.m_textures[tex].GetNameIndex()}";
    }

    /// Raw bytes as 4-byte words: a float where it looks like one, else hex; GUIDs of known types named.
    private static void Raw(byte[] data, string pad, StringBuilder report) {
        if (data.Length == 0) return;
        var guids = new List<string>();
        for (int o = 0; o + 8 <= data.Length; o += 4) {
            ulong v = BitConverter.ToUInt64(data, o);
            if (v == 0 || v == ulong.MaxValue) continue;
            if (GuidTypes.Contains(teResourceGUID.Type(v)) && teResourceGUID.Index(v) != 0 && (v >> 60) == 0 && IO.AssetExists(v)) {
                guids.Add($"@{o}: {G(v)}");
            }
        }
        if (guids.Count > 0) report.AppendLine($"{pad}guids: {string.Join(", ", guids)}");
        const int max = 768;
        var shown = data.Length > max ? data[..max] : data;
        for (int o = 0; o < shown.Length; o += 64) {
            var line = shown[o..Math.Min(o + 64, shown.Length)];
            report.AppendLine($"{pad}{o,5}: {Floats(line)}");
        }
        if (data.Length > max) report.AppendLine($"{pad}... ({data.Length - max} more bytes in raw/)");
    }

    private static string Floats(byte[] data) {
        var parts = new List<string>();
        for (int o = 0; o + 4 <= data.Length; o += 4) {
            float f = BitConverter.ToSingle(data, o);
            uint u = BitConverter.ToUInt32(data, o);
            if (u == 0) parts.Add("0");
            else if (float.IsFinite(f) && Math.Abs(f) >= 1e-4 && Math.Abs(f) < 1e6) parts.Add(f.ToString("0.####"));
            else parts.Add($"#{u:X8}");
        }
        for (int o = data.Length / 4 * 4; o < data.Length; o++) parts.Add($"b{data[o]:X2}");
        return string.Join(" ", parts);
    }
}
