PROCEDURE Main()
   LOCAL cScreen
   LOCAL aFrame

   IF ! RTUI_AVAILABLE()
      ? "Ratatui is unavailable:", RTUI_LAST_ERROR()
      ErrorLevel( 1 )
      RETURN
   ENDIF

   aFrame := RTUI_FRAME_NEW( 72, 15 )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 0, 0, 72, 11 ), "MinGW 64 demo", ;
      "Compiler: Harbour + MinGW-w64 x86_64" + Chr( 10 ) + ;
      "The same generic HRC1 command buffer is shared with Zig64.", ;
      RTUI_RGB( 240, 244, 255 ), RTUI_RGB( 18, 24, 38 ), ;
      RTUI_RGB( 114, 239, 221 ), 0, .T., .F., .T. )
   RTUI_FRAME_GAUGE( aFrame, RTUI_RECT( 0, 11, 72, 4 ), "Binding status", ;
      "Harbour command buffer -> Ratatui: OK", 1.0, ;
      RTUI_RGB( 80, 210, 170 ), RTUI_RGB( 25, 48, 50 ), ;
      RTUI_RGB( 80, 210, 170 ) )
   cScreen := RTUI_FRAME_RENDER( aFrame, .T. )

   IF cScreen == NIL
      ? "Render failed:", RTUI_LAST_ERROR()
      ErrorLevel( 2 )
      RETURN
   ENDIF

   IF ! RTUI_PRESENT( cScreen, .T. )
      ? "Native console presentation failed:", RTUI_LAST_ERROR()
      ErrorLevel( 3 )
      RETURN
   ENDIF
   ? "Loaded ABI version:", RTUI_ABI_VERSION()
RETURN
