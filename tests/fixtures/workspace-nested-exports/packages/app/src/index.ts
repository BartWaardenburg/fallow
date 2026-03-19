import { Card } from '@workspace/ui';
import { formatColor } from '@workspace/ui/utils';
import { Button } from '@workspace/ui/components/Button';

export const main = () => Card() + formatColor("red") + Button();
