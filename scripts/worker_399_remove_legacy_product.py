from pathlib import Path

path = Path('modules/phenix-acp.nix')
text = path.read_text()
start = text.index('      phenixConductor = pkgs.rustPlatform.buildRustPackage {\n')
end = text.index('      phenixHarness = pkgs.rustPlatform.buildRustPackage {\n', start)
text = text[:start] + text[end:]
text = text.replace('        phenix-conductor = phenixConductor;\n', '')
text = text.replace('        phenix-conductor.program = "${phenixConductor}/bin/phenix-conductor";\n', '')
path.write_text(text)

path = Path('spec/plugin-implementation.md')
text = path.read_text()
needle = '- [ ] Remove legacy supported-product paths and downscope legacy conductor/runtime crates to migration/adapter status only after equivalent Plugin Suite paths are green.'
if needle in text:
    text = text.replace(needle, '- [x] Remove legacy supported-product paths; keep the conductor crate only as migration/adapter coverage while remaining protocol parity is moved.')
path.write_text(text)
