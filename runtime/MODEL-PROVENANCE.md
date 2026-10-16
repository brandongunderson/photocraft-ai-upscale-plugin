# Included upscale model provenance

Verified on 2026-10-15. All model files are copied without byte changes from the official Upscayl repository at commit `a00d55fee90e0f9435d5eaa86e76700df8199af8`. Exact URLs and SHA-256 values are in [downloads.json](downloads.json). All entries marked `default: true` are included in the plugin package. A checksum verifies provenance and integrity; it does not certify image quality or grant a license.

## Newly included models

| Plugin model | Files under `models/` | Original creator | Model license |
| --- | --- | --- | --- |
| High Fidelity | `high-fidelity-4x.bin`, `high-fidelity-4x.param` | Helaman / Philip Hofmann (Phhofm) | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) |
| Remacri | `remacri-4x.bin`, `remacri-4x.param` | Foolhardy / FoolhardyVEVO | [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) — non-commercial |
| UltraMix Balanced | `ultramix-balanced-4x.bin`, `ultramix-balanced-4x.param` | Kim2091 | [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) — non-commercial |
| UltraSharp | `ultrasharp-4x.bin`, `ultrasharp-4x.param` | Kim2091 | [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) — non-commercial |

High Fidelity is Upscayl's name for **4xHFA2k**, as identified in its [pinned README credits](https://github.com/upscayl/upscayl/blob/a00d55fee90e0f9435d5eaa86e76700df8199af8/README.md#-credits). The author's [release](https://github.com/Phhofm/models/releases/tag/4xHFA2k) names Philip Hofmann and CC BY 4.0; the [model README at the release commit](https://github.com/Phhofm/models/blob/c664eb51194ec40fc9f4e258f0980681178e85c2/4xHFA2k/README.md) states its training on anime images with JPEG compression and blur. A copy is retained as [source/HFA2k-README.txt](source/HFA2k-README.txt). Upscayl's pinned attribution is retained as [source/Upscayl-README.md](source/Upscayl-README.md).

Upscayl attributes Remacri to Foolhardy and UltraMix Balanced/UltraSharp to Kim2091. In [Upscayl issue #198](https://github.com/upscayl/upscayl/issues/198), model author Kim2091 identifies all three as CC BY-NC-SA 4.0 and requests attribution; the maintainers acknowledge and add the credits. Model release records: [Remacri](https://openmodeldb.info/models/4x-Remacri), [UltraSharp](https://openmodeldb.info/models/4x-UltraSharp). The [pinned UltraMix Balanced NCNN distribution](https://github.com/upscayl/upscayl/blob/a00d55fee90e0f9435d5eaa86e76700df8199af8/resources/models/ultramix-balanced-4x.param) is attributed and licensed through the same upstream credits and author statement.

No additional training, interpolation, quantization or conversion was performed by this plugin project. The supplied files are the pre-existing Upscayl NCNN variants of those original models; their names and format differ from some original model downloads. This distribution retains those model-specific licenses and does not claim endorsement by the authors.

## Terms and redistribution

The full license texts are [LICENSE-CC-BY-4.0.txt](LICENSE-CC-BY-4.0.txt) and [LICENSE-CC-BY-NC-SA-4.0.txt](LICENSE-CC-BY-NC-SA-4.0.txt), retrieved from Creative Commons' official legal-code text URLs. Attribution, license notices, relevant source links and indications of changes must be retained as required by the applicable license. The three non-commercial models must not be presented as commercially licensed; obtain separate permission for uses outside those terms. If sharing adaptations, follow the applicable ShareAlike conditions. The models are supplied as-is without warranties, subject to the full license disclaimers.

The plugin's MIT source license and the upstream Upscayl repository license do not relicense model weights. Other components and the previously included Standard, Lite and Digital Art models retain their existing notices in [../THIRD-PARTY-NOTICES.md](../THIRD-PARTY-NOTICES.md).
