// Lance les tests du projet, sans connexion réseau (les API sont simulées).
// Écrit en Dart (installé avec Flutter) pour marcher pareil sous Windows,
// macOS et Linux, sans bash.
//
//   dart scripts/test.dart            # tout : Rust puis Flutter
//   dart scripts/test.dart rust       # backend Rust (cœur, API, extensions)
//   dart scripts/test.dart flutter    # toutes les apps et packages Flutter
//   dart scripts/test.dart tv         # app TV + packages partagés
//   dart scripts/test.dart remote     # télécommande + packages partagés
//
// Utilisé par `just test*` et par la CI (ci-tv, ci-remote, ci-rust) :
// même commande partout.
import 'dart:io';

/// Packages Flutter partagés (client HTTP, design system).
const sharedPackages = ['packages/client', 'packages/design_system'];

/// Apps et packages Flutter testés, dans l'ordre.
const flutterPackages = [...sharedPackages, 'apps/tv', 'apps/remote'];

Future<void> main(List<String> args) async {
  // Racine du dépôt = dossier parent de scripts/.
  final root = File.fromUri(Platform.script).parent.parent.path;
  final target = args.isEmpty ? 'all' : args.first;

  switch (target) {
    case 'rust':
      await rust(root);
    case 'flutter':
      await flutter(root, flutterPackages);
    case 'tv':
      await flutter(root, [...sharedPackages, 'apps/tv']);
    case 'remote':
      await flutter(root, [...sharedPackages, 'apps/remote']);
    case 'all':
      await rust(root);
      await flutter(root, flutterPackages);
    default:
      stderr.writeln(
          'Usage : dart scripts/test.dart [all|rust|flutter|tv|remote]');
      exit(2);
  }
  stdout.writeln('==> Tous les tests sont passés.');
}

Future<void> rust(String root) async {
  stdout.writeln('==> Rust : cargo test --workspace');
  await run('cargo', ['test', '--workspace'], root);
}

Future<void> flutter(String root, List<String> packages) async {
  stdout.writeln('==> Flutter : dépendances du workspace');
  await run('flutter', ['pub', 'get'], root);

  for (final package in packages) {
    final dir = '$root/$package';
    // Un package sans aucun fichier *_test.dart est signalé, pas en échec.
    if (!hasTests(Directory('$dir/test'))) {
      stdout.writeln('==> $package : aucun test (ignoré)');
      continue;
    }
    stdout.writeln('==> $package : flutter test');
    await run('flutter', ['test'], dir);
  }
}

bool hasTests(Directory dir) =>
    dir.existsSync() &&
    dir
        .listSync(recursive: true)
        .any((f) => f is File && f.path.endsWith('_test.dart'));

/// Lance une commande en affichant sa sortie ; arrête tout si elle échoue.
Future<void> run(String exe, List<String> args, String dir) async {
  final process = await Process.start(
    exe,
    args,
    workingDirectory: dir,
    mode: ProcessStartMode.inheritStdio,
    // flutter est un .bat sous Windows : il faut passer par le shell.
    runInShell: Platform.isWindows,
  );
  final code = await process.exitCode;
  if (code != 0) {
    stderr.writeln('==> Échec : $exe ${args.join(' ')} (dans $dir), code $code');
    exit(code);
  }
}
