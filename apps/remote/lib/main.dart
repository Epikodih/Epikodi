import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'app.dart';
import 'deps.dart';
import 'storage/remote_store.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // Télécommande : toujours en portrait.
  await SystemChrome.setPreferredOrientations([DeviceOrientation.portraitUp]);
  SystemChrome.setSystemUIOverlayStyle(SystemUiOverlayStyle.light);

  runApp(GloubiRemoteApp(deps: RemoteDeps(store: DeviceRemoteStore())));
}
