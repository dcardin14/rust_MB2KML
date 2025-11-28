# rust_MB2KML
Refactored my MB2KML from c++ into Rust


11/27/2025 DC:  Curve functionality

Example:
CRV L 200.00 34 26 06
CRV R 150.00 20 00 00

Description:
CRV → “this is a curve”
L / R → curve to the Left or Right
200.00 → radius (in the same units as your distances: feet, varas, chains, etc.)
34 26 06 → central angle Δ in DMS


======================================
 Type 2
 Example:
 CRV_RL L   1992.0  903.9
 Curve to the left defined by radius and arc length.  Radius is 1992.0.  Arc Length is 903.9.
 *The direction of the curve is inherited from the preceding line, not from the curve call itself.  That's why we don't incorporate chord bearing and distance here.

 ====================================
 Type 3
 Syntax:
 CRV_RDC  L|R  RADIUS  ΔD  ΔM  ΔS  CH_NS  CH_D  CH_M  CH_S  CH_EW  CHORD_DIST
 Example:
 CRV_RDC L 1992.0 34 26 06 N 45 00 00 E 1600.00
 Curve to the left defined by Radius, Central Angle (Delta), and chord
 Field	Description
 L / R	Direction of the curve (Left or Right)
 RADIUS	Radius of the curve (in selected distance units)
 ΔD ΔM ΔS	Central angle (Delta) in Degrees, Minutes, Seconds
 CH_NS	Chord bearing North/South (N or S)
 CH_D CH_M CH_S	Chord bearing angle in DMS
 CH_EW	Chord bearing East/West (E or W)
 CHORD_DIST	Straight-line distance from PC to PT (in distance units)
 How Orientation is Determined

In CRV_RDC, the curve’s position in space is controlled by the chord bearing, not by a previous tangent.

From geometry:

The tangent bearing at the Point of Curvature (PC) is computed as:

tangent_PC = chord_bearing ± (Δ / 2)


Where:

Left curves subtract half the delta

Right curves add half the delta

This defines the incoming bearing of the curve automatically.

How the Curve is Drawn

Internally, the engine:

Converts Radius to feet

Computes Δ in radians

Validates chord distance against geometry: