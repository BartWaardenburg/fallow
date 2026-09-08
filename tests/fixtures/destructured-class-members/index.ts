import { Foo, RestSource, DynamicSource } from './classes';

const foo = new Foo();
const { bar } = foo;
const alias = foo;
const { renamed: local, defaulted = 0, nested: { value }, ['computed']: selected } = alias;
console.log(bar, local, defaulted, value, selected);

const restSource = new RestSource();
const { first, ...rest } = restSource;
console.log(first, rest.second);

const dynamicSource = new DynamicSource();
const key = Math.random() > 0.5 ? 'first' : 'second';
const { [key]: dynamic } = dynamicSource;
console.log(dynamic);
