PROCEDURE Main()
   LOCAL cScreen
   LOCAL aFrame

   IF ! RTUI_AVAILABLE()
      ? "Ratatui is unavailable:", RTUI_LAST_ERROR()
      ErrorLevel( 1 )
      RETURN
   ENDIF

   aFrame := RTUI_FRAME_NEW( 72, 15 )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 0, 0, 72, 11 ), "Zig 64 demo", ;
      "Compiler: Harbour + Zig x86_64" + Chr( 10 ) + ;
      "Harbour defines this layout through the generic HRC1 command buffer.", ;
      RTUI_RGB( 240, 244, 255 ), RTUI_RGB( 18, 24, 38 ), ;
      RTUI_RGB( 114, 239, 221 ), 0, .T., .F., .T. )
   RTUI_FRAME_GAUGE( aFrame, RTUI_RECT( 0, 11, 72, 4 ), "Binding status", ;
      "Harbour command buffer -> Ratatui: OK", 1.0, ;
      RTUI_RGB( 255, 105, 180 ), RTUI_RGB( 38, 32, 55 ), ;
      RTUI_RGB( 255, 105, 180 ) )
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
