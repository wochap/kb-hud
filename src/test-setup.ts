// Enables React's act() environment for @testing-library/react and the
// createRoot-based DOM tests.
(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
