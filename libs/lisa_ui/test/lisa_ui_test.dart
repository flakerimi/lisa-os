import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:lisa_ui/lisa_ui.dart';

Widget harness(Widget child) => Directionality(
  textDirection: TextDirection.ltr,
  child: LisaTheme(tokens: LisaTokens.fallback, child: child),
);

void main() {
  testWidgets('LisaStreamText accumulates streamed tokens', (tester) async {
    final controller = StreamController<String>();
    await tester.pumpWidget(
      harness(LisaStreamText(stream: controller.stream, onStop: () {})),
    );

    controller.add('Hello ');
    await tester.pump();
    controller.add('world');
    await tester.pump();

    expect(find.textContaining('Hello world'), findsOneWidget);
    expect(find.text('Stop'), findsOneWidget, reason: 'streaming shows stop');

    await controller.close();
    await tester.pump();
    expect(find.text('Stop'), findsNothing, reason: 'done hides stop');
  });

  testWidgets('LisaStreamText renders provenance footnotes when done', (
    tester,
  ) async {
    final controller = StreamController<String>();
    await tester.pumpWidget(
      harness(
        LisaStreamText(
          stream: controller.stream,
          provenance: const ['file', 'screen'],
        ),
      ),
    );
    controller.add('answer');
    await controller.close();
    await tester.pump();

    expect(find.textContaining('⌁ file'), findsOneWidget);
    expect(find.textContaining('⌁ screen'), findsOneWidget);
  });

  testWidgets('ConsentChip fires allow and deny callbacks', (tester) async {
    var allowed = 0;
    var denied = 0;
    await tester.pumpWidget(
      harness(
        ConsentChip(
          scope: 'documents.read',
          onAllow: () => allowed++,
          onDeny: () => denied++,
        ),
      ),
    );

    expect(find.text('documents.read'), findsOneWidget);
    await tester.tap(find.text('Allow'));
    await tester.tap(find.text('Deny'));
    expect((allowed, denied), (1, 1));
  });

  test('lisaTheme derives from the violet seed, M3, Rubik', () {
    final theme = lisaTheme(Brightness.light);
    final expected = ColorScheme.fromSeed(
      seedColor: lisaSeedColor,
      brightness: Brightness.light,
    );
    expect(theme.colorScheme.primary, expected.primary);
    expect(theme.colorScheme.brightness, Brightness.light);
    expect(theme.useMaterial3, isTrue);
    expect(theme.textTheme.bodyMedium?.fontFamily, 'Rubik');

    final dark = lisaTheme(Brightness.dark);
    expect(dark.colorScheme.brightness, Brightness.dark);
    expect(
      dark.colorScheme.primary,
      ColorScheme.fromSeed(
        seedColor: lisaSeedColor,
        brightness: Brightness.dark,
      ).primary,
    );
  });

  testWidgets('LisaApp builds in light and dark mode', (tester) async {
    for (final (mode, want) in [
      (ThemeMode.light, Brightness.light),
      (ThemeMode.dark, Brightness.dark),
    ]) {
      late ThemeData seen;
      await tester.pumpWidget(
        LisaApp(
          themeMode: mode,
          home: Builder(
            builder: (context) {
              seen = Theme.of(context);
              return const SizedBox();
            },
          ),
        ),
      );
      // MaterialApp wraps the theme in an AnimatedTheme; let the
      // light→dark transition finish before asserting on it.
      await tester.pumpAndSettle();
      expect(seen.colorScheme.brightness, want);
      expect(
        seen.colorScheme.primary,
        ColorScheme.fromSeed(
          seedColor: lisaSeedColor,
          brightness: want,
        ).primary,
      );
      expect(seen.textTheme.bodyMedium?.fontFamily, 'Rubik');
    }
  });

  testWidgets('LisaScaffold renders a working button', (tester) async {
    var taps = 0;
    await tester.pumpWidget(
      LisaApp(
        home: LisaScaffold(
          title: 'Demo',
          body: Center(
            child: ElevatedButton(
              onPressed: () => taps++,
              child: const Text('Go'),
            ),
          ),
        ),
      ),
    );

    expect(find.text('Demo'), findsOneWidget);
    await tester.tap(find.text('Go'));
    expect(taps, 1);
  });

  testWidgets('LisaCard renders its child in a Card', (tester) async {
    await tester.pumpWidget(
      const LisaApp(
        home: LisaScaffold(body: LisaCard(child: Text('inside'))),
      ),
    );

    expect(find.text('inside'), findsOneWidget);
    expect(find.byType(Card), findsOneWidget);
  });
}
