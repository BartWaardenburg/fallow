const lang = getLang();
const locale = import(`./locales/${lang}`);
const page = 'home';
const pageModule = import('./pages/' + page);
const utils = import('./utils');

console.log(locale, pageModule, utils);

function getLang(): string { return 'en'; }
