// require.context — Webpack pattern, should make matching files reachable
const icons = require.context('./icons', false);

console.log(icons);
