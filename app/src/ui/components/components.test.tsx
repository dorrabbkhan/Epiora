// Component-set smoke tests (Task 7): keyboard activation, dialog
// focus management and semantic table markup.
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import { describe, expect, it, vi } from 'vitest';
import {
  Button,
  Cell,
  Column,
  Dialog,
  DialogTrigger,
  Input,
  Label,
  Modal,
  Row,
  Table,
  TableBody,
  TableHeader,
  TextField,
} from './index';
import { Heading } from 'react-aria-components';

describe('Button', () => {
  it('button_is_keyboard_activatable', async () => {
    const user = userEvent.setup();
    const onPress = vi.fn();
    render(<Button onPress={onPress}>Press me</Button>);
    const button = screen.getByRole('button', { name: 'Press me' });
    button.focus();
    await user.keyboard('{Enter}');
    expect(onPress).toHaveBeenCalledTimes(1);
    await user.keyboard(' ');
    expect(onPress).toHaveBeenCalledTimes(2);
  });
});

describe('Dialog', () => {
  it('dialog_traps_and_returns_focus', async () => {
    const user = userEvent.setup();
    render(
      <DialogTrigger>
        <Button>Open dialog</Button>
        <Modal isDismissable>
          <Dialog>
            <Heading slot="title">Details</Heading>
            <TextField>
              <Label>Name</Label>
              <Input />
            </TextField>
            <Button slot="close">Close</Button>
          </Dialog>
        </Modal>
      </DialogTrigger>,
    );
    const trigger = screen.getByRole('button', { name: 'Open dialog' });
    trigger.focus();
    await user.keyboard('{Enter}');

    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveAccessibleName('Details');
    // Focus moved inside and stays inside when tabbing.
    expect(dialog.contains(document.activeElement)).toBe(true);
    for (let i = 0; i < 6; i += 1) {
      await user.tab();
      expect(dialog.contains(document.activeElement)).toBe(true);
    }

    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    // RAC restores focus to the trigger on the next frame.
    await waitFor(() => expect(trigger).toHaveFocus());
  });
});

describe('Table', () => {
  it('table_renders_caption_and_th_scope', () => {
    render(
      <Table caption="Oswego cases" aria-label="Oswego cases">
        <TableHeader>
          <Column isRowHeader>Name</Column>
          <Column>Age</Column>
        </TableHeader>
        <TableBody>
          <Row>
            <Cell>Ada</Cell>
            <Cell>36</Cell>
          </Row>
        </TableBody>
      </Table>,
    );
    const table = screen.getByRole('grid', { name: 'Oswego cases' });
    const caption = table.querySelector('caption');
    expect(caption).not.toBeNull();
    expect(caption).toHaveTextContent('Oswego cases');
    // Column headers are real <th> elements; the row-header cell is
    // scoped by the rowheader role.
    expect(table.querySelectorAll('thead th')).toHaveLength(2);
    expect(screen.getAllByRole('columnheader')).toHaveLength(2);
    const rowHeader = screen.getByRole('rowheader');
    expect(rowHeader).toHaveTextContent('Ada');
  });
});
