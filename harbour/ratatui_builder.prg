/* Harbour-side builder for the HRC1 command-buffer interface.
 * Application state and composition stay in Harbour; Rust only validates
 * and executes these generic widget commands. */

#define HRT_MAGIC       "HRC1"
#define HRT_VERSION     1

#define HRT_CLEAR       1
#define HRT_BLOCK       2
#define HRT_PARAGRAPH   3
#define HRT_TABS        4
#define HRT_LIST        5
#define HRT_GAUGE       6
#define HRT_TABLE       7
#define HRT_SPARKLINE   8
#define HRT_BARCHART    9
#define HRT_CHART       10

/* Compact paragraph style mask.  The byte is translated to Ratatui's
 * Modifier bitflags by the Rust adapter.  Keep these values stable: the
 * old lBold argument is encoded as RTUI_MOD_BOLD for compatibility. */
#define RTUI_MOD_NONE          0
#define RTUI_MOD_BOLD          1
#define RTUI_MOD_DIM           2
#define RTUI_MOD_ITALIC        4
#define RTUI_MOD_UNDERLINE     8
#define RTUI_MOD_BLINK         16
#define RTUI_MOD_REVERSE       32
#define RTUI_MOD_CROSSED       64
#define RTUI_MOD_RAPID_BLINK   128
#define RTUI_MOD_MASK_MAX      255

/* Ratatui-compatible aliases that read naturally in Harbour code. */
#define RTUI_MOD_UNDERLINED    RTUI_MOD_UNDERLINE
#define RTUI_MOD_REVERSED      RTUI_MOD_REVERSE
#define RTUI_MOD_CROSSED_OUT   RTUI_MOD_CROSSED
#define RTUI_MOD_STRIKETHROUGH RTUI_MOD_CROSSED

FUNCTION RTUI_RGB( nRed, nGreen, nBlue )
RETURN { nRed, nGreen, nBlue }

FUNCTION RTUI_RECT( nX, nY, nWidth, nHeight )
RETURN { nX, nY, nWidth, nHeight }

FUNCTION RTUI_FRAME_NEW( nWidth, nHeight )
RETURN { nWidth, nHeight, {} }

FUNCTION RTUI_FRAME_BYTES( aFrame )
   LOCAL cCommands := ""
   LOCAL cCommand

   FOR EACH cCommand IN aFrame[ 3 ]
      cCommands += cCommand
   NEXT

RETURN HRT_MAGIC + HRT_U16( HRT_VERSION ) + ;
   HRT_U16( aFrame[ 1 ] ) + HRT_U16( aFrame[ 2 ] ) + ;
   HRT_U16( Len( aFrame[ 3 ] ) ) + cCommands

FUNCTION RTUI_FRAME_RENDER( aFrame, lAnsi )
RETURN RTUI_RENDER_COMMANDS( RTUI_FRAME_BYTES( aFrame ), lAnsi )

FUNCTION RTUI_FRAME_CLEAR( aFrame, aRect, aBackground )
   HRT_AddCommand( aFrame, HRT_CLEAR, ;
      HRT_RectBytes( aRect ) + HRT_Color( aBackground ) )
RETURN aFrame

FUNCTION RTUI_FRAME_BLOCK( aFrame, aRect, cTitle, aBorder, aBackground )
   HRT_AddCommand( aFrame, HRT_BLOCK, HRT_RectBytes( aRect ) + ;
      HRT_Text( cTitle ) + HRT_Color( aBorder ) + HRT_Color( aBackground ) )
RETURN aFrame

FUNCTION RTUI_FRAME_PARAGRAPH( aFrame, aRect, cTitle, cText, ;
   aForeground, aBackground, aBorder, nAlignment, lWrap, lBold, lBordered, ;
   nModifiers )

   /* nModifiers is optional.  Existing callers that pass lBold continue to
    * produce the same HRC1 payload (0 or RTUI_MOD_BOLD). */
   LOCAL nStyleMask := HRT_ModifierMask( nModifiers, lBold )

   LOCAL cPayload := HRT_RectBytes( aRect ) + HRT_U8( iif( lBordered, 1, 0 ) ) + ;
      HRT_Text( cTitle ) + HRT_Text( cText ) + HRT_Color( aForeground ) + ;
      HRT_Color( aBackground ) + HRT_Color( aBorder ) + HRT_U8( nAlignment ) + ;
      HRT_U8( iif( lWrap, 1, 0 ) ) + HRT_U8( nStyleMask )

   HRT_AddCommand( aFrame, HRT_PARAGRAPH, cPayload )
RETURN aFrame

FUNCTION RTUI_FRAME_TABS( aFrame, aRect, aItems, nSelected, ;
   aForeground, aBackground, aSelectedForeground, aSelectedBackground, aBorder )

   LOCAL cPayload := HRT_RectBytes( aRect ) + HRT_U16( nSelected ) + ;
      HRT_U16( Len( aItems ) )
   LOCAL cItem

   FOR EACH cItem IN aItems
      cPayload += HRT_Text( cItem )
   NEXT
   cPayload += HRT_Color( aForeground ) + HRT_Color( aBackground ) + ;
      HRT_Color( aSelectedForeground ) + HRT_Color( aSelectedBackground ) + ;
      HRT_Color( aBorder )
   HRT_AddCommand( aFrame, HRT_TABS, cPayload )
RETURN aFrame

FUNCTION RTUI_FRAME_LIST( aFrame, aRect, cTitle, aItems, nSelected, ;
   lActive, cMarker, aBackground, aBorder, aSelectedForeground, aSelectedBackground )

   LOCAL cPayload := HRT_RectBytes( aRect ) + HRT_Text( cTitle ) + ;
      HRT_U16( nSelected ) + HRT_U8( iif( lActive, 1, 0 ) ) + ;
      HRT_Text( cMarker ) + HRT_U16( Len( aItems ) )
   LOCAL aItem

   FOR EACH aItem IN aItems
      cPayload += HRT_Text( aItem[ 1 ] ) + HRT_Color( aItem[ 2 ] )
   NEXT
   cPayload += HRT_Color( aBackground ) + HRT_Color( aBorder ) + ;
      HRT_Color( aSelectedForeground ) + HRT_Color( aSelectedBackground )
   HRT_AddCommand( aFrame, HRT_LIST, cPayload )
RETURN aFrame

FUNCTION RTUI_FRAME_GAUGE( aFrame, aRect, cTitle, cLabel, nRatio, ;
   aForeground, aBackground, aBorder )

   LOCAL cPayload := HRT_RectBytes( aRect ) + HRT_Text( cTitle ) + ;
      HRT_Text( cLabel ) + HRT_U16( Round( Max( 0, Min( 1, nRatio ) ) * 10000, 0 ) ) + ;
      HRT_Color( aForeground ) + HRT_Color( aBackground ) + HRT_Color( aBorder )

   HRT_AddCommand( aFrame, HRT_GAUGE, cPayload )
RETURN aFrame

FUNCTION RTUI_FRAME_TABLE( aFrame, aRect, cTitle, aWidths, aHeaders, aRows, ;
   nSelected, lActive, cMarker, aBackground, aBorder, aHeaderForeground, ;
   aHeaderBackground, aSelectedForeground, aSelectedBackground )

   LOCAL cPayload := HRT_RectBytes( aRect ) + HRT_Text( cTitle ) + ;
      HRT_U16( nSelected ) + HRT_U8( iif( lActive, 1, 0 ) ) + ;
      HRT_Text( cMarker ) + HRT_U16( Len( aWidths ) )
   LOCAL nValue
   LOCAL cValue
   LOCAL aRow

   FOR EACH nValue IN aWidths
      cPayload += HRT_U16( nValue )
   NEXT
   FOR EACH cValue IN aHeaders
      cPayload += HRT_Text( cValue )
   NEXT
   cPayload += HRT_U16( Len( aRows ) )
   FOR EACH aRow IN aRows
      FOR EACH cValue IN aRow
         cPayload += HRT_Text( cValue )
      NEXT
   NEXT
   cPayload += HRT_Color( aBackground ) + HRT_Color( aBorder ) + ;
      HRT_Color( aHeaderForeground ) + HRT_Color( aHeaderBackground ) + ;
      HRT_Color( aSelectedForeground ) + HRT_Color( aSelectedBackground )
   HRT_AddCommand( aFrame, HRT_TABLE, cPayload )
RETURN aFrame

FUNCTION RTUI_FRAME_SPARKLINE( aFrame, aRect, cTitle, aValues, ;
   aForeground, aBackground, aBorder )

   LOCAL cPayload := HRT_RectBytes( aRect ) + HRT_Text( cTitle ) + ;
      HRT_U16( Len( aValues ) )
   LOCAL nValue

   FOR EACH nValue IN aValues
      cPayload += HRT_U16( nValue )
   NEXT
   cPayload += HRT_Color( aForeground ) + HRT_Color( aBackground ) + ;
      HRT_Color( aBorder )
   HRT_AddCommand( aFrame, HRT_SPARKLINE, cPayload )
RETURN aFrame

FUNCTION RTUI_FRAME_BARCHART( aFrame, aRect, cTitle, aBars, nBarWidth, nBarGap, ;
   aForeground, aValueForeground, aBackground, aBorder )

   LOCAL cPayload := HRT_RectBytes( aRect ) + HRT_Text( cTitle ) + ;
      HRT_U16( nBarWidth ) + HRT_U16( nBarGap ) + HRT_U16( Len( aBars ) )
   LOCAL aBar

   FOR EACH aBar IN aBars
      cPayload += HRT_Text( aBar[ 1 ] ) + HRT_U16( aBar[ 2 ] )
   NEXT
   cPayload += HRT_Color( aForeground ) + HRT_Color( aValueForeground ) + ;
      HRT_Color( aBackground ) + HRT_Color( aBorder )
   HRT_AddCommand( aFrame, HRT_BARCHART, cPayload )
RETURN aFrame

FUNCTION RTUI_FRAME_CHART( aFrame, aRect, cTitle, nXMin, nXMax, nYMin, nYMax, ;
   aSeries, aBackground, aBorder )

   LOCAL cPayload := HRT_RectBytes( aRect ) + HRT_Text( cTitle ) + ;
      HRT_I32( Round( nXMin * 100, 0 ) ) + HRT_I32( Round( nXMax * 100, 0 ) ) + ;
      HRT_I32( Round( nYMin * 100, 0 ) ) + HRT_I32( Round( nYMax * 100, 0 ) ) + ;
      HRT_U16( Len( aSeries ) )
   LOCAL aData
   LOCAL aPoint

   FOR EACH aData IN aSeries
      cPayload += HRT_Text( aData[ 1 ] ) + HRT_Color( aData[ 2 ] ) + ;
         HRT_U16( Len( aData[ 3 ] ) )
      FOR EACH aPoint IN aData[ 3 ]
         cPayload += HRT_I32( Round( aPoint[ 1 ] * 100, 0 ) ) + ;
            HRT_I32( Round( aPoint[ 2 ] * 100, 0 ) )
      NEXT
   NEXT
   cPayload += HRT_Color( aBackground ) + HRT_Color( aBorder )
   HRT_AddCommand( aFrame, HRT_CHART, cPayload )
RETURN aFrame

STATIC PROCEDURE HRT_AddCommand( aFrame, nOpcode, cPayload )
   AAdd( aFrame[ 3 ], HRT_U8( nOpcode ) + HRT_U8( 0 ) + ;
      HRT_U32( Len( cPayload ) ) + cPayload )
RETURN

STATIC FUNCTION HRT_RectBytes( aRect )
RETURN HRT_U16( aRect[ 1 ] ) + HRT_U16( aRect[ 2 ] ) + ;
   HRT_U16( aRect[ 3 ] ) + HRT_U16( aRect[ 4 ] )

STATIC FUNCTION HRT_Color( aColor )
RETURN HRT_U8( aColor[ 1 ] ) + HRT_U8( aColor[ 2 ] ) + HRT_U8( aColor[ 3 ] )

STATIC FUNCTION HRT_ModifierMask( nModifiers, lBold )
   LOCAL nMask

   IF ValType( nModifiers ) == "N"
      nMask := Int( nModifiers )
   ELSE
      nMask := iif( lBold, RTUI_MOD_BOLD, RTUI_MOD_NONE )
   ENDIF

RETURN Max( RTUI_MOD_NONE, Min( RTUI_MOD_MASK_MAX, nMask ) )

STATIC FUNCTION HRT_Text( cText )
RETURN HRT_U32( Len( cText ) ) + cText

STATIC FUNCTION HRT_I32( nValue )
   IF nValue < 0
      nValue += 4294967296
   ENDIF
RETURN HRT_U32( nValue )

STATIC FUNCTION HRT_U32( nValue )
   nValue := Int( nValue )
RETURN HRT_U8( nValue ) + HRT_U8( Int( nValue / 256 ) ) + ;
   HRT_U8( Int( nValue / 65536 ) ) + HRT_U8( Int( nValue / 16777216 ) )

STATIC FUNCTION HRT_U16( nValue )
   nValue := Int( nValue )
RETURN HRT_U8( nValue ) + HRT_U8( Int( nValue / 256 ) )

STATIC FUNCTION HRT_U8( nValue )
RETURN Chr( Int( nValue ) % 256 )
