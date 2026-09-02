#include "../../harbour/ratatui.ch"
#include "inkey.ch"

PROCEDURE Main()
   LOCAL cFrame
   LOCAL nKey
   LOCAL nTick := 0
   LOCAL nTreeSelected := 0
   LOCAL nTableSelected := 0
   LOCAL nFocus := 0
   LOCAL nMenu := 0
   LOCAL nMenuItem := 0
   LOCAL nChecked := 63
   LOCAL nExpanded := 3
   LOCAL nBit
   LOCAL nBitValue
   LOCAL nGroupBit
   LOCAL nPosition
   LOCAL aVisible
   LOCAL aTreeBits := { -1, 0, -1, 1, 2, 3, -1, 4, 5 }
   LOCAL lPaused := .F.
   LOCAL lMenuOpen := .T.
   LOCAL lRunning := .T.
   LOCAL lAutoClose := hb_GetEnv( "HB_RATATUI_AUTOCLOSE" ) == "1"
   LOCAL lBenchmark := hb_GetEnv( "HB_RATATUI_BENCHMARK" ) == "1"
   LOCAL lAnsi := ! lAutoClose

   SetMode( 52, 122 )

   IF ! RTUI_AVAILABLE()
      ? "Ratatui is unavailable:", RTUI_LAST_ERROR()
      ErrorLevel( 1 )
      RETURN
   ENDIF

   IF lBenchmark
      RunRenderBenchmark()
      RETURN
   ENDIF

   DO WHILE lRunning
      cFrame := BuildShowcaseFrame( nTick, nTreeSelected, nTableSelected, ;
         nFocus, nMenu, nMenuItem, nChecked, nExpanded, lMenuOpen, lAnsi )
      IF cFrame == NIL
         ? "Showcase render failed:", RTUI_LAST_ERROR()
         ErrorLevel( 2 )
         RETURN
      ENDIF

      IF ! RTUI_PRESENT( cFrame, lAnsi )
         ? "Native console presentation failed:", RTUI_LAST_ERROR()
         ErrorLevel( 3 )
         RETURN
      ENDIF
      nKey := Inkey( 0.08 )
      aVisible := VisibleTreeNodes( nExpanded )

      DO CASE
      CASE nKey == Asc( "q" ) .OR. nKey == Asc( "Q" )
         lRunning := .F.
      CASE nKey == K_ESC
         IF lMenuOpen
            lMenuOpen := .F.
         ELSE
            lRunning := .F.
         ENDIF
      CASE nKey == K_TAB .OR. nKey == Asc( "m" ) .OR. nKey == Asc( "M" )
         lMenuOpen := ! lMenuOpen
      CASE nKey == K_F6
         nFocus := 1 - nFocus
         lMenuOpen := .F.
      CASE nKey == K_LEFT
         nMenu := iif( nMenu == 0, 3, nMenu - 1 )
         nMenuItem := 0
         lMenuOpen := .T.
      CASE nKey == K_RIGHT
         nMenu := ( nMenu + 1 ) % 4
         nMenuItem := 0
         lMenuOpen := .T.
      CASE nKey == K_UP
         IF lMenuOpen
            nMenuItem := iif( nMenuItem == 0, 3, nMenuItem - 1 )
         ELSEIF nFocus == 0
            nPosition := TreeNodePosition( aVisible, nTreeSelected )
            IF nPosition > 1
               nTreeSelected := aVisible[ nPosition - 1 ]
            ENDIF
         ELSE
            nTableSelected := Max( 0, nTableSelected - 1 )
         ENDIF
      CASE nKey == K_DOWN
         IF lMenuOpen
            nMenuItem := ( nMenuItem + 1 ) % 4
         ELSEIF nFocus == 0
            nPosition := TreeNodePosition( aVisible, nTreeSelected )
            IF nPosition < Len( aVisible )
               nTreeSelected := aVisible[ nPosition + 1 ]
            ENDIF
         ELSE
            nTableSelected := Min( 3, nTableSelected + 1 )
         ENDIF
      CASE ( nKey == Asc( "+" ) .OR. nKey == Asc( "-" ) ) .AND. ;
           ! lMenuOpen .AND. nFocus == 0
         nGroupBit := iif( nTreeSelected == 2, 0, ;
            iif( nTreeSelected == 6, 1, -1 ) )
         IF nGroupBit >= 0
            nBitValue := 2 ^ nGroupBit
            IF nKey == Asc( "+" ) .AND. ;
               Int( nExpanded / nBitValue ) % 2 == 0
               nExpanded += nBitValue
            ELSEIF nKey == Asc( "-" ) .AND. ;
               Int( nExpanded / nBitValue ) % 2 == 1
               nExpanded -= nBitValue
            ENDIF
         ENDIF
      CASE nKey == Asc( " " )
         IF ! lMenuOpen .AND. nFocus == 0
            nBit := aTreeBits[ nTreeSelected + 1 ]
            IF nBit >= 0
               nBitValue := 2 ^ nBit
               IF Int( nChecked / nBitValue ) % 2 == 1
                  nChecked -= nBitValue
               ELSE
                  nChecked += nBitValue
               ENDIF
            ENDIF
         ENDIF
      CASE nKey == K_ENTER .AND. lMenuOpen
         lMenuOpen := .F.
      CASE nKey == Asc( "p" ) .OR. nKey == Asc( "P" )
         lPaused := ! lPaused
      ENDCASE

      IF ! lPaused
         nTick++
      ENDIF
      IF lAutoClose .AND. nTick >= 5
         lRunning := .F.
      ENDIF
   ENDDO

RETURN

STATIC PROCEDURE RunRenderBenchmark()
   LOCAL nIterations := Val( hb_GetEnv( "HB_RATATUI_BENCHMARK_ITERATIONS" ) )
   LOCAL nIndex
   LOCAL nStarted
   LOCAL nCommandMs
   LOCAL nLegacyMs
   LOCAL cReport
   LOCAL cReportPath := hb_GetEnv( "HB_RATATUI_BENCHMARK_FILE" )

   IF nIterations <= 0
      nIterations := 300
   ENDIF

   FOR nIndex := 1 TO 20
      BuildShowcaseFrame( nIndex, 0, 0, 0, 0, 0, 63, 3, .F., .F. )
      RTUI_SHOWCASE_TREE( nIndex, 0, 0, 0, 0, 0, 63, 3, ;
         .F., 120, 38, .F. )
   NEXT

   nStarted := Seconds()
   FOR nIndex := 1 TO nIterations
      BuildShowcaseFrame( nIndex, 0, 0, 0, 0, 0, 63, 3, .F., .F. )
   NEXT
   nCommandMs := ( Seconds() - nStarted ) * 1000 / nIterations

   nStarted := Seconds()
   FOR nIndex := 1 TO nIterations
      RTUI_SHOWCASE_TREE( nIndex, 0, 0, 0, 0, 0, 63, 3, ;
         .F., 120, 38, .F. )
   NEXT
   nLegacyMs := ( Seconds() - nStarted ) * 1000 / nIterations

   cReport := "COMMAND_BUFFER_MS_PER_FRAME=" + ;
      AllTrim( Str( nCommandMs, 12, 3 ) ) + hb_eol() + ;
      "LEGACY_RUST_MS_PER_FRAME=" + AllTrim( Str( nLegacyMs, 12, 3 ) ) + hb_eol()
   IF nLegacyMs > 0
      cReport += "COMMAND_BUFFER_RATIO=" + ;
         AllTrim( Str( nCommandMs / nLegacyMs, 12, 3 ) ) + hb_eol()
   ENDIF
   IF Empty( cReportPath )
      cReportPath := "command_benchmark.txt"
   ENDIF
   MemoWrit( cReportPath, cReport )
RETURN

STATIC FUNCTION BuildShowcaseFrame( nTick, nTreeSelected, nTableSelected, ;
   nFocus, nMenu, nMenuItem, nChecked, nExpanded, lMenuOpen, lAnsi )

   LOCAL aBackground := RTUI_RGB( 18, 24, 38 )
   LOCAL aBlack := RTUI_RGB( 0, 0, 0 )
   LOCAL aWhite := RTUI_RGB( 240, 244, 255 )
   LOCAL aGray := RTUI_RGB( 95, 110, 135 )
   LOCAL aCyan := RTUI_RGB( 114, 239, 221 )
   LOCAL aBlue := RTUI_RGB( 88, 125, 255 )
   LOCAL aGreen := RTUI_RGB( 80, 210, 170 )
   LOCAL aPink := RTUI_RGB( 255, 105, 180 )
   LOCAL aYellow := RTUI_RGB( 255, 220, 105 )
   LOCAL aPurple := RTUI_RGB( 190, 120, 255 )
   LOCAL aSelection := RTUI_RGB( 42, 63, 92 )
   LOCAL aGaugeBackground := RTUI_RGB( 38, 32, 55 )
   LOCAL aFrame := RTUI_FRAME_NEW( 120, 48 )
   LOCAL aVisible := VisibleTreeNodes( nExpanded )
   LOCAL aTreeItems := {}
   LOCAL aRows
   LOCAL aHeaders := { "COMPONENT", "STATE", "LATENCY", "HEALTH" }
   LOCAL aWidths := { 34, 23, 20, 23 }
   LOCAL aThroughput := {}
   LOCAL aLatency := {}
   LOCAL aSparkBase := { 12, 18, 31, 47, 62, 74, 79, 71, 55, 39, 24, 17, ;
      21, 36, 58, 76, 88, 83, 67, 45, 29, 19, 25, 43, 66, 81, 91, 78, 54, 32 }
   LOCAL aSpark := {}
   LOCAL aBars := { { "Zig64", 94 }, { "M64", 87 }, { "M32", 72 }, { "ABI", 100 } }
   LOCAL aSeries
   LOCAL aMenuItems
   LOCAL aMenuX := { 2, 15, 29, 44 }
   LOCAL nPhase := nTick / 7.0
   LOCAL nPulse := ( Sin( nPhase ) + 1.0 ) * 0.5
   LOCAL nCpu := Max( 0, Min( 1, 0.42 + nPulse * 0.46 ) )
   LOCAL nMemory := Max( 0, Min( 1, 0.71 + Cos( nPhase / 2.0 ) * 0.08 ) )
   LOCAL nIndex
   LOCAL nNode
   LOCAL cMark
   LOCAL cTreeTitle := iif( nFocus == 0 .AND. ! lMenuOpen, ;
      "▶ Tree: hierarchy + ticks", "Tree: hierarchy + ticks" )
   LOCAL cTableTitle := iif( nFocus == 1 .AND. ! lMenuOpen, ;
      "▶ Table + live data", "Table + live data" )

   FOR EACH nNode IN aVisible
      DO CASE
      CASE nNode == 0
         AAdd( aTreeItems, { "◆ Workspace", aCyan } )
      CASE nNode == 1
         AAdd( aTreeItems, { "  " + TreeCheck( nChecked, 0 ) + " Harbour VM", aGreen } )
      CASE nNode == 2
         cMark := iif( TreeBitSet( nExpanded, 0 ), "-", "+" )
         AAdd( aTreeItems, { "  [" + cMark + "] Toolchains", aCyan } )
      CASE nNode == 3
         AAdd( aTreeItems, { "      " + TreeCheck( nChecked, 1 ) + " Zig64", aGreen } )
      CASE nNode == 4
         AAdd( aTreeItems, { "      " + TreeCheck( nChecked, 2 ) + " MinGW64", aGreen } )
      CASE nNode == 5
         AAdd( aTreeItems, { "      " + TreeCheck( nChecked, 3 ) + " MinGW32", aPurple } )
      CASE nNode == 6
         cMark := iif( TreeBitSet( nExpanded, 1 ), "-", "+" )
         AAdd( aTreeItems, { "  [" + cMark + "] Widgets", aCyan } )
      CASE nNode == 7
         AAdd( aTreeItems, { "      " + TreeCheck( nChecked, 4 ) + " Charts", aYellow } )
      CASE nNode == 8
         AAdd( aTreeItems, { "      " + TreeCheck( nChecked, 5 ) + " Tables", aYellow } )
      ENDCASE
   NEXT

   aRows := { ;
      { "Harbour VM", "RUNNING", "1.8 ms", "99.99%" }, ;
      { "Ratatui FFI", "RUNNING", "0.3 ms", "100.0%" }, ;
      { "DBF index", "SYNCING", "4.7 ms", "99.95%" }, ;
      { "Events", AllTrim( Str( 9420 + nTick * 17 ) ), "0.6 ms", "live" } }

   FOR nIndex := 0 TO 49
      AAdd( aThroughput, { nIndex, ;
         58.0 + Sin( nIndex / 4.2 + nPhase ) * 22.0 + Cos( nIndex / 2.7 ) * 7.0 } )
      AAdd( aLatency, { nIndex, 24.0 + Cos( nIndex / 5.5 + nPhase * 0.7 ) * 11.0 } )
   NEXT
   aSeries := { { "throughput", aPink, aThroughput }, ;
      { "latency", aCyan, aLatency } }

   FOR nIndex := 1 TO Len( aSparkBase )
      AAdd( aSpark, aSparkBase[ ( nIndex + nTick - 1 ) % Len( aSparkBase ) + 1 ] )
   NEXT

   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 0, 0, 120, 3 ), "", ;
      "◆ HARBOUR  ↔  RATATUI     Harbour command buffer     frame " + ;
      StrZero( nTick, 6 ), aPink, aBackground, aPink, 1, .F., .T., .T. )
   RTUI_FRAME_TABS( aFrame, RTUI_RECT( 0, 3, 120, 3 ), ;
      { " File ", " View ", " Tools ", " Help " }, nMenu, ;
      aGray, aBackground, aBlack, aCyan, aBlue )
   RTUI_FRAME_LIST( aFrame, RTUI_RECT( 0, 6, 34, 11 ), cTreeTitle, ;
      aTreeItems, TreeNodePosition( aVisible, nTreeSelected ) - 1, ;
      nFocus == 0 .AND. ! lMenuOpen, "▶ ", aBackground, aBlue, aWhite, aSelection )
   RTUI_FRAME_GAUGE( aFrame, RTUI_RECT( 0, 17, 34, 4 ), "CPU", ;
      AllTrim( Str( Round( nCpu * 100, 0 ) ) ) + "%", nCpu, ;
      aPink, aGaugeBackground, aPink )
   RTUI_FRAME_GAUGE( aFrame, RTUI_RECT( 0, 21, 34, 4 ), "Memory", ;
      AllTrim( Str( Round( nMemory * 160, 1 ) / 10 ) ) + " / 16 GiB", nMemory, ;
      aGreen, RTUI_RGB( 25, 48, 50 ), aGreen )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 0, 25, 34, 10 ), "Capabilities", ;
      "✓ UTF-8: Здравей • こんにちは" + Chr( 10 ) + ;
      "✓ TrueColor RGB via Win32 VT" + Chr( 10 ) + ;
      "✓ UI/state defined in Harbour", aWhite, aBackground, aPurple, 0, .T., .F., .T. )
   RTUI_FRAME_TABLE( aFrame, RTUI_RECT( 35, 6, 85, 10 ), cTableTitle, ;
      aWidths, aHeaders, aRows, nTableSelected, nFocus == 1 .AND. ! lMenuOpen, ;
      "▸ ", aBackground, aCyan, aBlack, aCyan, aYellow, RTUI_RGB( 45, 45, 65 ) )
   RTUI_FRAME_CHART( aFrame, RTUI_RECT( 35, 17, 49, 18 ), "Braille Chart", ;
      0, 49, 0, 100, aSeries, aBackground, aPurple )
   RTUI_FRAME_SPARKLINE( aFrame, RTUI_RECT( 85, 17, 35, 8 ), "Sparkline", ;
      aSpark, aCyan, aBackground, aCyan )
   RTUI_FRAME_BARCHART( aFrame, RTUI_RECT( 85, 26, 35, 9 ), ;
      "BarChart / toolchains", aBars, 6, 2, aPurple, aWhite, aBackground, aPurple )
   AddRichFeatures( aFrame, aBackground, aWhite, aGray, aCyan, aGreen, ;
      aPink, aYellow, aPurple )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 0, 45, 120, 3 ), "", ;
      "M/TAB menu   F6 focus   ARROWS move   +/- fold   SPACE tick   P pause   Q/ESC close", ;
      aCyan, aBackground, aGray, 1, .F., .T., .T. )

   IF lMenuOpen
      DO CASE
      CASE nMenu == 0
         aMenuItems := { "New dashboard", "Open project…", "Save snapshot", "Exit" }
      CASE nMenu == 1
         aMenuItems := { "Overview", "Services", "Performance", "Unicode" }
      CASE nMenu == 2
         aMenuItems := { "ABI inspector", "DBF monitor", "Theme editor", "Diagnostics" }
      OTHERWISE
         aMenuItems := { "Keyboard help", "FFI reference", "About Ratatui", "About Harbour" }
      ENDCASE
      RTUI_FRAME_CLEAR( aFrame, RTUI_RECT( aMenuX[ nMenu + 1 ], 6, 32, 7 ), aBackground )
      RTUI_FRAME_LIST( aFrame, RTUI_RECT( aMenuX[ nMenu + 1 ], 6, 32, 7 ), ;
         "Menu", MenuRows( aMenuItems, aWhite ), nMenuItem, .T., "› ", ;
         aBackground, aCyan, aBlack, aCyan )
   ENDIF

RETURN RTUI_FRAME_RENDER( aFrame, lAnsi )

STATIC PROCEDURE AddRichFeatures( aFrame, aBackground, aWhite, aGray, ;
   aCyan, aGreen, aPink, aYellow, aPurple )

   LOCAL nGradientX := 44
   LOCAL nGradientWidth := 74
   LOCAL nIndex

   RTUI_FRAME_BLOCK( aFrame, RTUI_RECT( 0, 35, 120, 10 ), ;
      "Rich features", aPink, aBackground )

   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 2, 36, 9, 1 ), "", "Colors", ;
      aPink, aBackground, aPink, 0, .F., .F., .F., RTUI_MOD_BOLD )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 12, 36, 29, 1 ), "", ;
      "✓ 4-bit color", aGreen, aBackground, aGreen, 0, .F., .F., .F., ;
      RTUI_MOD_BOLD )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 12, 37, 29, 1 ), "", ;
      "✓ 8-bit color", RTUI_RGB( 100, 180, 255 ), aBackground, aCyan, ;
      0, .F., .F., .F., RTUI_MOD_BOLD )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 12, 38, 29, 1 ), "", ;
      "✓ Truecolor (16.7 million)", aPurple, aBackground, aPurple, ;
      0, .F., .F., .F., RTUI_MOD_BOLD )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 12, 39, 29, 1 ), "", ;
      "✓ Automatic color conversion", aCyan, aBackground, aCyan, ;
      0, .F., .F., .F., RTUI_MOD_BOLD )

   FOR nIndex := 0 TO nGradientWidth - 1
      RTUI_FRAME_CLEAR( aFrame, ;
         RTUI_RECT( nGradientX + nIndex, 36, 1, 5 ), ;
         RainbowColor( nIndex, nGradientWidth ) )
   NEXT
   RTUI_FRAME_PARAGRAPH( aFrame, ;
      RTUI_RECT( nGradientX, 41, nGradientWidth, 1 ), "", ;
      "24-bit RGB gradient generated entirely by Harbour", ;
      aWhite, aBackground, aWhite, 1, .F., .F., .F. )

   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 2, 42, 9, 1 ), "", "Styles", ;
      aPink, aBackground, aPink, 0, .F., .F., .F., RTUI_MOD_BOLD )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 12, 42, 7, 1 ), "", "bold", ;
      aWhite, aBackground, aWhite, 0, .F., .F., .F., RTUI_MOD_BOLD )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 20, 42, 5, 1 ), "", "dim", ;
      aGray, aBackground, aGray, 0, .F., .F., .F., RTUI_MOD_DIM )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 26, 42, 8, 1 ), "", "italic", ;
      aPurple, aBackground, aPurple, 0, .F., .F., .F., RTUI_MOD_ITALIC )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 35, 42, 11, 1 ), "", "underline", ;
      aCyan, aBackground, aCyan, 0, .F., .F., .F., RTUI_MOD_UNDERLINE )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 47, 42, 15, 1 ), "", ;
      "strikethrough", aPink, aBackground, aPink, 0, .F., .F., .F., ;
      RTUI_MOD_CROSSED )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 63, 42, 10, 1 ), "", "reverse", ;
      RTUI_RGB( 0, 0, 0 ), aYellow, aYellow, 1, .F., .F., .F., ;
      RTUI_MOD_REVERSE )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 74, 42, 7, 1 ), "", "blink", ;
      RTUI_RGB( 255, 80, 80 ), aBackground, aPink, 0, .F., .F., .F., ;
      RTUI_MOD_BLINK )
RETURN

STATIC FUNCTION RainbowColor( nIndex, nCount )
   LOCAL nScaled := nIndex * 6.0 / Max( 1, nCount - 1 )
   LOCAL nSegment := Min( 5, Int( nScaled ) )
   LOCAL nPart := nScaled - nSegment
   LOCAL nRed := 0
   LOCAL nGreen := 0
   LOCAL nBlue := 0

   DO CASE
   CASE nSegment == 0
      nRed := 255
      nGreen := Round( 255 * nPart, 0 )
   CASE nSegment == 1
      nRed := Round( 255 * ( 1 - nPart ), 0 )
      nGreen := 255
   CASE nSegment == 2
      nGreen := 255
      nBlue := Round( 255 * nPart, 0 )
   CASE nSegment == 3
      nGreen := Round( 255 * ( 1 - nPart ), 0 )
      nBlue := 255
   CASE nSegment == 4
      nRed := Round( 255 * nPart, 0 )
      nBlue := 255
   OTHERWISE
      nRed := 255
      nBlue := Round( 255 * ( 1 - nPart ), 0 )
   ENDCASE

RETURN RTUI_RGB( nRed, nGreen, nBlue )

STATIC FUNCTION MenuRows( aItems, aColor )
   LOCAL aRows := {}
   LOCAL cItem

   FOR EACH cItem IN aItems
      AAdd( aRows, { "  " + cItem, aColor } )
   NEXT
RETURN aRows

STATIC FUNCTION TreeCheck( nChecked, nBit )
RETURN iif( TreeBitSet( nChecked, nBit ), "☑", "☐" )

STATIC FUNCTION TreeBitSet( nMask, nBit )
RETURN Int( nMask / ( 2 ^ nBit ) ) % 2 == 1

STATIC FUNCTION VisibleTreeNodes( nExpanded )
   LOCAL aNodes := { 0, 1, 2 }

   IF Int( nExpanded / 1 ) % 2 == 1
      AAdd( aNodes, 3 )
      AAdd( aNodes, 4 )
      AAdd( aNodes, 5 )
   ENDIF

   AAdd( aNodes, 6 )
   IF Int( nExpanded / 2 ) % 2 == 1
      AAdd( aNodes, 7 )
      AAdd( aNodes, 8 )
   ENDIF

RETURN aNodes

STATIC FUNCTION TreeNodePosition( aNodes, nNode )
   LOCAL nPosition

   FOR nPosition := 1 TO Len( aNodes )
      IF aNodes[ nPosition ] == nNode
         RETURN nPosition
      ENDIF
   NEXT

RETURN 1
