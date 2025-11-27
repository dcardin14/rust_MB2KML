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