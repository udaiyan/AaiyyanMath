# Example user-authored engineering helpers.
# Import explicitly from a trusted Python block, for example:
#   import engineering as eng
#   print(eng.beam_deflection_uniform(10e3, 3.0, 210e9, 8.5e-6))


def beam_deflection_uniform(w, L, E, I):
    return 5.0 * w * L ** 4 / (384.0 * E * I)


def beam_moment_uniform(w, L, x):
    return w * x * (L - x) / 2.0


def beam_stress(M, y, I):
    return M * y / I


def reynolds(rho, v, d, mu):
    return rho * v * d / mu


def darcy_pressure_loss(f, L, d, rho, v):
    return f * (L / d) * rho * v ** 2 / 2.0
