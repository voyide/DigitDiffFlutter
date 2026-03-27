import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:universal_sequencer/rust/api.dart' as rust;
import 'package:universal_sequencer/rust/solver.dart';
import 'package:path_provider/path_provider.dart';
import 'package:share_plus/share_plus.dart';
import 'dart:io';
import 'rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const SequencerApp());
}

class SequencerApp extends StatelessWidget {
  const SequencerApp({super.key});
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF0F0F13),
        cardColor: const Color(0xFF1A1A24),
        primaryColor: const Color(0xFF00E5FF),
      ),
      home: const MainInterface(),
    );
  }
}

class MainInterface extends StatefulWidget {
  const MainInterface({super.key});
  @override
  State<MainInterface> createState() => _MainInterfaceState();
}

class _MainInterfaceState extends State<MainInterface> {
  // Ruleset
  int ruleLhs = 0; int ruleRhs1 = 0; int ruleRhs2 = 1; String logicRhs = "NONE";
  String customLHS = "Math.abs(xi - x_next)";
  String customRHS1 = "val == di"; String customRHS2 = "val <= di";
  
  // Post Process
  String postType = "NONE"; int postK = 2; String postGridType = "BASE2";
  int postR = 10; int postC = 10; int postT = 0; int postMc = 4;
  
  // Viewer
  int baseB = 3; String viewScale = "2"; int viewR = 100; int viewC = 100;
  String offsetP = "0"; int modM = 4; int kOffset = 1;
  
  // Anim
  String animMode = "modeA"; int animStart = 2; int animEnd = 6;
  
  ui.Image? renderedGrid;
  bool isRendering = false;
  String sysLogs = "System Initialized Native Engine...\n";

  void log(String msg) {
    setState(() => sysLogs += "[${DateTime.now().toIso8601String().split('T')[1].substring(0,8)}] $msg\n");
  }

  rust.Config _buildConfig(int renderR, int renderC) {
    return rust.Config(
      b: baseB, p: offsetP, m: modM, kOffset: kOffset,
      lhs: ruleLhs, rhs1: ruleRhs1, rhs2: ruleRhs2, logic: logicRhs,
      customLhs: customLHS, customRhs1: customRHS1, customRhs2: customRHS2,
      postType: postType, postK: postK, gridR: postR, gridC: postC,
      targetT: postT, modMc: postMc, renderR: renderR, renderC: renderC, startN: "0",
    );
  }

  Future<void> _triggerRender() async {
    if (isRendering) return;
    setState(() => isRendering = true);
    
    int actR = viewScale == "CUSTOM" ? viewR : (baseB * baseB); // simplified scale logic for demo
    int actC = viewScale == "CUSTOM" ? viewC : (baseB * baseB);
    actR = actR > 1000 ? 1000 : actR; actC = actC > 1000 ? 1000 : actC;

    try {
      final Uint8List rgba = await rust.compileGrid(cfg: _buildConfig(actR, actC));
      ui.decodeImageFromPixels(rgba, actC, actR, ui.PixelFormat.rgba8888, (img) {
        setState(() { renderedGrid = img; isRendering = false; });
      });
    } catch (e) {
      log("Error: $e"); setState(() => isRendering = false);
    }
  }

  Future<void> _exportHiRes() async {
    log("Starting High Res Export (4000x4000 limit)...");
    setState(() => isRendering = true);
    int actR = viewScale == "CUSTOM" ? viewR : (baseB * baseB); 
    int actC = viewScale == "CUSTOM" ? viewC : (baseB * baseB);
    actR = actR > 4000 ? 4000 : actR; actC = actC > 4000 ? 4000 : actC;

    try {
      final Uint8List rgba = await rust.compileGrid(cfg: _buildConfig(actR, actC));
      // Save PNG via Flutter Image package or encode to file
      log("High Res buffer generated. Ready to save.");
    } catch (e) { log("HR Error: $e"); }
    setState(() => isRendering = false);
  }

  Future<void> _exportGif() async {
    log("Compiling GIF native ($animMode)...");
    setState(() => isRendering = true);
    try {
      final Uint8List gifBytes = await rust.compileGifAnimation(
        cfg: _buildConfig(400, 400), mode: animMode, framesStart: animStart, framesEnd: animEnd
      );
      final dir = await getTemporaryDirectory();
      final file = File('${dir.path}/sequence.gif');
      await file.writeAsBytes(gifBytes);
      Share.shareXFiles([XFile(file.path)], text: 'Generated native Math Sequence');
      log("GIF successfully exported.");
    } catch (e) { log("GIF Error: $e"); }
    setState(() => isRendering = false);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text("Universal Sequencer Native")),
      body: Row(
        children:[
          // Left Sidebar - Exhaustive Inputs
          Expanded(
            flex: 1,
            child: ListView(
              padding: const EdgeInsets.all(16),
              children:[
                const Text("Ruleset Logic", style: TextStyle(color: Color(0xFF00E5FF), fontSize: 18)),
                DropdownButtonFormField<int>(
                  value: ruleLhs,
                  items: List.generate(19, (i) => DropdownMenuItem(value: i, child: Text("LHS Rule $i"))),
                  onChanged: (v) { setState(() => ruleLhs = v!); _triggerRender(); },
                ),
                if (ruleLhs == 13) TextField(onChanged: (v) => customLHS = v, decoration: const InputDecoration(labelText: "Custom Rhai JS")),
                
                DropdownButtonFormField<int>(
                  value: ruleRhs1,
                  items: List.generate(13, (i) => DropdownMenuItem(value: i, child: Text("RHS1 Rule $i"))),
                  onChanged: (v) { setState(() => ruleRhs1 = v!); _triggerRender(); },
                ),
                if (ruleRhs1 == 12) TextField(onChanged: (v) => customRHS1 = v, decoration: const InputDecoration(labelText: "Custom Rhai JS 1")),

                const Divider(),
                const Text("Post-Processing", style: TextStyle(color: Color(0xFF00E5FF), fontSize: 18)),
                DropdownButtonFormField<String>(
                  value: postType,
                  items:["NONE", "ITERATE", "C_SEQ", "D_SEQ"].map((e) => DropdownMenuItem(value: e, child: Text(e))).toList(),
                  onChanged: (v) { setState(() => postType = v!); _triggerRender(); },
                ),
                
                const Divider(),
                const Text("Viewer Options", style: TextStyle(color: Color(0xFF00E5FF), fontSize: 18)),
                Row(children:[
                  Expanded(child: Slider(min: 2, max: 20, value: baseB.toDouble(), label: "Base b", onChanged: (v) { setState(() => baseB = v.toInt()); _triggerRender(); })),
                  Text("B: $baseB")
                ]),
                Row(children:[
                  Expanded(child: Slider(min: 2, max: 20, value: modM.toDouble(), label: "Mod M", onChanged: (v) { setState(() => modM = v.toInt()); _triggerRender(); })),
                  Text("M: $modM")
                ]),
                
                const Divider(),
                const Text("System Logs", style: TextStyle(color: Colors.grey)),
                Container(
                  height: 100, color: Colors.black, padding: const EdgeInsets.all(8),
                  child: SingleChildScrollView(child: Text(sysLogs, style: const TextStyle(color: Colors.green, fontFamily: 'monospace', fontSize: 10))),
                ),
                const SizedBox(height: 10),
                ElevatedButton(onPressed: _exportHiRes, child: const Text("Export High Res")),
                ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFFFF4081)), onPressed: _exportGif, child: const Text("Compile GIF")),
              ],
            ),
          ),
          
          // Right Canvas
          Expanded(
            flex: 2,
            child: Stack(
              alignment: Alignment.center,
              children:[
                if (renderedGrid != null)
                  InteractiveViewer(
                    maxScale: 10.0,
                    child: CustomPaint(size: const Size(600, 600), painter: GridPainter(renderedGrid!)),
                  ),
                if (isRendering) const CircularProgressIndicator(),
              ],
            ),
          )
        ],
      ),
    );
  }
}

class GridPainter extends CustomPainter {
  final ui.Image image;
  GridPainter(this.image);
  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawImageRect(image, Rect.fromLTWH(0,0, image.width.toDouble(), image.height.toDouble()), Rect.fromLTWH(0, 0, size.width, size.height), Paint()..filterQuality = FilterQuality.none);
  }
  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}
