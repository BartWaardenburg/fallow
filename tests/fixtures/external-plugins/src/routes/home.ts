// Route file — should be an entry point via external plugin
// `default` and `loader` exports should be considered used
export default function HomePage() {
  return 'Home';
}

export const loader = () => {
  return { title: 'Home' };
};

export const unusedRouteExport = () => 'not used';
