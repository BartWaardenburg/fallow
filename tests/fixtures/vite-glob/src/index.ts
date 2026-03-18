// import.meta.glob — Vite pattern, should make all matching files reachable
const modules = import.meta.glob('./components/*.ts');

console.log(modules);
