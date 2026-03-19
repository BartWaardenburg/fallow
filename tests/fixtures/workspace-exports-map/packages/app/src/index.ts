import { Button } from '@workspace/ui';
import { formatColor } from '@workspace/ui/utils';
import { clamp } from '@workspace/ui/helpers';
export const main = () => Button() + formatColor("red") + clamp(5, 0, 10);
