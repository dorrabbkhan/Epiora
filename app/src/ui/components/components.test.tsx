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

  // The caption is portaled into the <table> and moved with
  // insertBefore; re-renders must keep it the first child without
  // React removeChild/insertBefore errors.
  it('table_caption_stays_first_child_across_rerenders', () => {
    function Grid({ rows, caption }: { rows: string[]; caption: string }) {
      return (
        <Table caption={caption}>
          <TableHeader>
            <Column isRowHeader>Name</Column>
          </TableHeader>
          <TableBody>
            {rows.map((r) => (
              <Row key={r}>
                <Cell>{r}</Cell>
              </Row>
            ))}
          </TableBody>
        </Table>
      );
    }
    const { container, rerender, unmount } = render(
      <Grid rows={['a', 'b']} caption="one" />,
    );
    const table = () => {
      const el = container.querySelector('table');
      if (el === null) throw new Error('no <table> rendered');
      return el;
    };
    const firstChild = () => table().firstElementChild;
    expect(firstChild()?.tagName).toBe('CAPTION');

    // Caption text changes: same element updated in place.
    const captionEl = firstChild();
    rerender(<Grid rows={['a', 'b']} caption="two" />);
    expect(firstChild()?.tagName).toBe('CAPTION');
    expect(firstChild()).toBe(captionEl);
    expect(firstChild()).toHaveTextContent('two');

    // Rows removed then added.
    rerender(<Grid rows={['a']} caption="two" />);
    expect(firstChild()?.tagName).toBe('CAPTION');
    rerender(<Grid rows={['a', 'b', 'c']} caption="two" />);
    expect(firstChild()?.tagName).toBe('CAPTION');
    expect(firstChild()).toHaveTextContent('two');

    // Unmounting does not throw (the portal detaches cleanly).
    expect(() => unmount()).not.toThrow();
  });
});
