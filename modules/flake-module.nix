{ inputs, ... }:

{
  perSystem =
    { system, ... }:
    {
      phenixWrapped = {
        phenix = inputs.self.packages.${system}.phenix;
        conductor = inputs.self.packages.${system}.phenix-kernel;
        harness = inputs.self.packages.${system}.phenix-harness;
        runtime = inputs.self.packages.${system}.phenix-harness;
        stitch = inputs.self.packages.${system}.stitch;
        stitchMcp = inputs.self.packages.${system}.stitch-mcp;
      };
    };
}
