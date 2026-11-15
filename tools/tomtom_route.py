"""Offline TomTom drafts: explicit UIMapIDs and manual paste, never game input."""
import argparse
import hashlib
import json
import math
import re
import unicodedata

from tools.sync_exports import absolute, ordinary

MAX_BYTES = 64 * 1024
MAX_STEPS = 64
STEP_KEYS = {'ui_map_id', 'x_percent', 'y_percent', 'label'}


def encoded(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'), allow_nan=False)


def safe_text(value):
    if (type(value) is not str or not value.strip() or len(value) > 80
            or any(c == '|' or unicodedata.category(c)[0] == 'C'
                   or unicodedata.category(c) in ('Zl', 'Zp') for c in value)
            or len(value.encode('utf-8')) > 80):
        raise ValueError('Title and labels must be 1 to 80 safe UTF-8 bytes')
    return ' '.join(value.split())


def integer(value, maximum):
    if type(value) is not int or not 1 <= value <= maximum:
        raise ValueError('Expected a positive bounded integer')
    return value


def coordinate(value):
    if type(value) not in (int, float) or not 0 <= value <= 100 or not math.isfinite(value):
        raise ValueError('Coordinates must be finite numbers from 0 to 100 percent')
    return value


def display_labels(pin):
    """Predict the two numeric-locale rules in the qualified v4.4.1 parser."""
    spaced = re.sub(r'([0-9])[.,] ([0-9])', r'\1 \2', pin)
    variants = {'dot': re.sub(r'([0-9]),([0-9])', r'\1.\2', spaced),
                'comma': re.sub(r'([0-9])\.([0-9])', r'\1,\2', spaced)}
    return variants if any(label != pin for label in variants.values()) else None


def prepare_route(title, steps, active_step=1, mode='stepwise'):
    """Return a deterministic draft. Array order is a plan, not arrow execution order."""
    title = safe_text(title)
    if type(steps) is not list or not 1 <= len(steps) <= MAX_STEPS:
        raise ValueError('A route must contain 1 to 64 ordered steps')
    integer(active_step, len(steps))
    if type(mode) is not str or mode not in ('stepwise', 'all_pins'):
        raise ValueError('Mode must be stepwise or all_pins')
    clean = []
    for step in steps:
        if type(step) is not dict or set(step) != STEP_KEYS:
            raise ValueError('Each step requires only ui_map_id, x_percent, y_percent and label')
        clean.append(dict(ui_map_id=integer(step['ui_map_id'], 4294967295),
                          x_percent=coordinate(step['x_percent']),
                          y_percent=coordinate(step['y_percent']), label=safe_text(step['label'])))
    route_id = hashlib.sha256(encoded(dict(title=title, steps=clean)).encode('utf-8')).hexdigest()
    manifest, commands = [], []
    for number, step in enumerate(clean, 1):
        # Keep fixed decimals: TomTom's slash parser must never see exponent notation.
        x = format(step['x_percent'] or 0, '.5f')
        y = format(step['y_percent'] or 0, '.5f')
        pin = f'{number:02d}/{len(clean):02d} [{route_id}] {title}: {step["label"]}'
        variants = display_labels(pin)
        manifest.append(dict(number=number, **step, pin_label=pin,
                             parser_may_change_pin_label=variants is not None,
                             **({'display_pin_label_variants': variants} if variants else {}),
                             rendered_x_percent=x, rendered_y_percent=y,
                             coordinates_rounded=(float(x) != step['x_percent']
                                                  or float(y) != step['y_percent'])))
        if mode == 'all_pins' or number == active_step:
            commands.append(f'/tway #{step["ui_map_id"]} {x} {y} {pin}')
    result = dict(route_id=route_id, title=title, mode=mode, active_step=active_step,
                  manifest=manifest, manual_import_text='\n'.join(commands),
                  draft_only=True, waypoint_creation_performed=False,
                  automatic_movement=False, auto_advance=False, arrow_order_enforced=False,
                  quest_completion_checked=False, native_coordinates_verified=False,
                  addon_runtime_verified=False, accessibility_verified=False,
                  instructions='Review the UIMapIDs and percent coordinates. Open /ttpaste and paste the text manually. '
                  'Stepwise exports only the selected step; select each later step manually. '
                  'All-pins preserves text order, but TomTom can select the closest pin. '
                  'Existing pins remain. No arrival or quest completion is checked.',
                  label_precision='pin_label is the paste text. Different display predictions for TomTom v4.4.1 '
                  'numeric locales are explicit; actual runtime labels remain unverified.',
                  coordinate_precision='Five fractional percent digits; requested and rendered values are in each manifest step.')
    # Account for the text-only MCP content envelope and JSON escaping, not just the raw draft.
    content = {'content': [{'type': 'text', 'text': encoded(result)}], 'isError': False}
    if len(encoded(content).encode('utf-8')) > MAX_BYTES - 1024:
        raise ValueError('Route response exceeds 64 KiB transport budget; shorten labels or split the route')
    return result


def prepare_request(value):
    if (type(value) is not dict or not {'title', 'steps'} <= set(value)
            or set(value) - {'title', 'steps', 'active_step', 'mode'}):
        raise ValueError('Expected only title, steps, optional active_step and mode')
    return prepare_route(**value)


def input_schema():
    return {'type': 'object', 'additionalProperties': False, 'required': ['title', 'steps'],
            'properties': {'title': {'type': 'string', 'minLength': 1, 'maxLength': 80},
                'active_step': {'type': 'integer', 'minimum': 1, 'maximum': MAX_STEPS, 'default': 1},
                'mode': {'type': 'string', 'enum': ['stepwise', 'all_pins'], 'default': 'stepwise'},
                'steps': {'type': 'array', 'minItems': 1, 'maxItems': MAX_STEPS,
                    'items': {'type': 'object', 'additionalProperties': False,
                        'required': sorted(STEP_KEYS), 'properties': {
                            'ui_map_id': {'type': 'integer', 'minimum': 1, 'maximum': 4294967295},
                            'x_percent': {'type': 'number', 'minimum': 0, 'maximum': 100},
                            'y_percent': {'type': 'number', 'minimum': 0, 'maximum': 100},
                            'label': {'type': 'string', 'minLength': 1, 'maxLength': 80}}}}}}


def serve():
    import asyncio
    from mcp.server.lowlevel import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import TextContent, Tool, ToolAnnotations
    server = Server('Azeroth Offline TomTom Route Drafts')

    @server.list_tools()
    async def list_tools():
        return [Tool(name='prepare_tomtom_route', inputSchema=input_schema(),
                     description='Prepare an OFFLINE ordered route manifest and manual TomTom paste text. '
                     'Explicit UIMapIDs only; no provider area ID conversions. Stepwise defaults to one pin. '
                     'No game input, navigation, auto-advance, quest checks or verified native coordinates.',
                     annotations=ToolAnnotations(readOnlyHint=True, destructiveHint=False,
                                                 idempotentHint=True, openWorldHint=False))]

    # Manual strict validation also bounds errors; SDK schema errors echo arbitrary input.
    @server.call_tool(validate_input=False)
    async def call_tool(name, arguments):
        if name != 'prepare_tomtom_route':
            raise ValueError('Unknown route tool')
        try:
            return [TextContent(type='text', text=encoded(prepare_request(arguments)))]
        except (ValueError, TypeError, OverflowError):
            raise ValueError('Invalid or oversized route draft') from None

    async def run():
        async with stdio_server() as (read, write):
            await server.run(read, write, server.create_initialization_options())
    asyncio.run(run())


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('Duplicate JSON keys are not supported')
        result[key] = value
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    actions = parser.add_subparsers(dest='action', required=True)
    actions.add_parser('serve')
    actions.add_parser('prepare').add_argument('--input', required=True)
    args = parser.parse_args()
    if args.action == 'serve':
        serve()
        return
    try:
        path = ordinary(absolute(args.input))
        if not path.is_file() or path.stat().st_nlink != 1:
            raise ValueError('Input must be an ordinary single-link file')
        with path.open('rb') as stream:
            payload = stream.read(MAX_BYTES + 1)
        if len(payload) > MAX_BYTES:
            raise ValueError('Input exceeds 64 KiB')
        value = json.loads(payload, object_pairs_hook=unique_object)
        print(encoded(prepare_request(value)))
    except (OSError, ValueError, TypeError, RecursionError):
        parser.error('Invalid route input; use an absolute ordinary JSON file within 64 KiB')


if __name__ == '__main__':
    main()
