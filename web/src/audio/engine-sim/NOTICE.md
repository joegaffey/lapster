# Engine_Sim

Vendored from https://github.com/YigitSalihEmecen/Engine_Sim at commit
`c3ff61031050f1fc280a864f4bb445f30aeaa0a3`.

The upstream repository declares MIT in its README and `package.json` but did
not contain a license file at the time of vendoring. The MIT text is included
in `LICENSE` for attribution and compliance.

Local changes: `EngineSim.workletURL` lets the host point the combustion
worklet at the versioned Pages asset. `EngineSim.updateExternal()` and
`Drivetrain.updateExternal()` drive the synthesis graph from the host vehicle's
RPM, speed, and gear instead of integrating the vendored drivetrain.
