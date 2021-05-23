EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A4 11693 8268
encoding utf-8
Sheet 1 1
Title ""
Date ""
Rev ""
Comp ""
Comment1 ""
Comment2 ""
Comment3 ""
Comment4 ""
$EndDescr
$Comp
L Device:R R1
U 1 1 60A2ACE9
P 3580 3270
F 0 "R1" H 3650 3316 50  0000 L CNN
F 1 "510R" H 3650 3225 50  0000 L CNN
F 2 "Resistor_THT:R_Axial_DIN0204_L3.6mm_D1.6mm_P5.08mm_Horizontal" V 3510 3270 50  0001 C CNN
F 3 "~" H 3580 3270 50  0001 C CNN
	1    3580 3270
	1    0    0    -1  
$EndComp
$Comp
L Device:D_Bridge_+AA- DB1
U 1 1 60A2C5B1
P 3150 3000
F 0 "DB1" H 3320 3260 50  0000 L CNN
F 1 "D_Bridge_+AA-" H 3494 2955 50  0001 L CNN
F 2 "Diode_THT:Diode_Bridge_32.0x5.6x17.0mm_P10.0mm_P7.5mm" H 3150 3000 50  0001 C CNN
F 3 "~" H 3150 3000 50  0001 C CNN
	1    3150 3000
	1    0    0    -1  
$EndComp
Wire Wire Line
	3450 3000 3580 3000
Wire Wire Line
	3580 3000 3580 3120
Wire Wire Line
	3580 3420 3580 3660
$Comp
L MCU_Microchip_PIC12:PIC12C508-IP U3
U 1 1 60A28317
P 8290 3580
F 0 "U3" H 8290 4361 50  0001 C CNN
F 1 "PIC12F508" H 7990 4100 50  0000 C CNN
F 2 "Package_DIP:DIP-8_W7.62mm" H 8890 4230 50  0001 C CNN
F 3 "http://www.ti.com/lit/ds/symlink/lm393.pdf" H 8290 3580 50  0001 C CNN
	1    8290 3580
	-1   0    0    -1  
$EndComp
$Comp
L Comparator:LM2903 U2
U 1 1 60A29EC7
P 6530 3580
F 0 "U2" H 6530 3947 50  0001 C CNN
F 1 "LM2903" H 6530 3855 50  0000 C CNN
F 2 "Package_DIP:DIP-8_W7.62mm_Socket" H 6530 3580 50  0001 C CNN
F 3 "http://www.ti.com/lit/ds/symlink/lm393.pdf" H 6530 3580 50  0001 C CNN
	1    6530 3580
	1    0    0    -1  
$EndComp
$Comp
L Comparator:LM2903 U2
U 3 1 60A3AB47
P 5810 3960
F 0 "U2" H 5810 4327 50  0001 C CNN
F 1 "LM2903" H 5370 3950 50  0000 L CNN
F 2 "Package_DIP:DIP-8_W7.62mm_Socket" H 5810 3960 50  0001 C CNN
F 3 "http://www.ti.com/lit/ds/symlink/lm393.pdf" H 5810 3960 50  0001 C CNN
	3    5810 3960
	1    0    0    -1  
$EndComp
$Comp
L Device:CP C1
U 1 1 60A43C73
P 3580 4010
F 0 "C1" H 3698 4056 50  0000 L CNN
F 1 "150uF" H 3698 3965 50  0000 L CNN
F 2 "Capacitor_THT:CP_Radial_D6.3mm_P2.50mm" H 3618 3860 50  0001 C CNN
F 3 "~" H 3580 4010 50  0001 C CNN
	1    3580 4010
	1    0    0    -1  
$EndComp
Wire Wire Line
	3580 3660 3580 3860
Connection ~ 3580 3660
Wire Wire Line
	3580 4260 3580 4160
Wire Wire Line
	3680 3660 3580 3660
Wire Wire Line
	3980 3960 3980 4260
Connection ~ 3980 4260
Wire Wire Line
	3580 4260 3980 4260
$Comp
L Regulator_Linear:L7805 U1
U 1 1 60A28F76
P 3980 3660
F 0 "U1" H 3980 3902 50  0001 C CNN
F 1 "7805" H 3980 3811 50  0000 C CNN
F 2 "Package_TO_SOT_THT:TO-220F-3_Horizontal_TabDown" H 4005 3510 50  0001 L CIN
F 3 "http://www.ti.com/lit/ds/symlink/lm393.pdf" H 3980 3610 50  0001 C CNN
	1    3980 3660
	1    0    0    -1  
$EndComp
$Comp
L Device:C C3
U 1 1 60A4FBDB
P 5030 3260
F 0 "C3" H 5145 3306 50  0000 L CNN
F 1 "100nF" H 5145 3215 50  0000 L CNN
F 2 "Capacitor_THT:C_Axial_L3.8mm_D2.6mm_P7.50mm_Horizontal" H 5068 3110 50  0001 C CNN
F 3 "~" H 5030 3260 50  0001 C CNN
	1    5030 3260
	1    0    0    -1  
$EndComp
$Comp
L Device:R R2
U 1 1 60A518A5
P 5030 3960
F 0 "R2" H 5100 4006 50  0000 L CNN
F 1 "2.2K" H 5100 3915 50  0000 L CNN
F 2 "Resistor_THT:R_Axial_DIN0204_L3.6mm_D1.6mm_P5.08mm_Horizontal" V 4960 3960 50  0001 C CNN
F 3 "~" H 5030 3960 50  0001 C CNN
	1    5030 3960
	1    0    0    -1  
$EndComp
Wire Wire Line
	3580 3000 5030 3000
Connection ~ 3580 3000
Wire Wire Line
	5030 3110 5030 3000
Wire Wire Line
	5030 3410 5030 3480
Wire Wire Line
	5030 4110 5030 4260
Text GLabel 4590 3660 2    50   Output ~ 0
+5V
$Comp
L Device:R R4
U 1 1 60A59332
P 5930 4110
F 0 "R4" H 6000 4156 50  0000 L CNN
F 1 "10K" H 6000 4065 50  0000 L CNN
F 2 "Resistor_THT:R_Axial_DIN0204_L3.6mm_D1.6mm_P5.08mm_Horizontal" V 5860 4110 50  0001 C CNN
F 3 "~" H 5930 4110 50  0001 C CNN
	1    5930 4110
	1    0    0    -1  
$EndComp
$Comp
L Device:R R3
U 1 1 60A5A0B8
P 5930 3810
F 0 "R3" H 6000 3856 50  0000 L CNN
F 1 "100K" H 6000 3765 50  0000 L CNN
F 2 "Resistor_THT:R_Axial_DIN0204_L3.6mm_D1.6mm_P5.08mm_Horizontal" V 5860 3810 50  0001 C CNN
F 3 "~" H 5930 3810 50  0001 C CNN
	1    5930 3810
	1    0    0    -1  
$EndComp
Wire Wire Line
	6230 3480 5030 3480
Connection ~ 5030 3480
Wire Wire Line
	5030 3480 5030 3810
Wire Wire Line
	5930 4260 5710 4260
Wire Wire Line
	2840 3000 2840 4260
Wire Wire Line
	2840 4260 3580 4260
Connection ~ 3580 4260
$Comp
L power:GND #PWR0101
U 1 1 60A641F6
P 2840 4430
F 0 "#PWR0101" H 2840 4180 50  0001 C CNN
F 1 "GND" H 2845 4257 50  0000 C CNN
F 2 "" H 2840 4430 50  0001 C CNN
F 3 "" H 2840 4430 50  0001 C CNN
	1    2840 4430
	1    0    0    -1  
$EndComp
Wire Wire Line
	2840 4430 2840 4260
Connection ~ 2840 4260
$Comp
L Transistor_BJT:PN2222A Q1
U 1 1 60A69B95
P 7040 3950
F 0 "Q1" H 7231 3996 50  0000 L CNN
F 1 "PN2222A" H 7231 3905 50  0000 L CNN
F 2 "Package_TO_SOT_THT:TO-92_Inline" H 7240 3875 50  0001 L CIN
F 3 "https://www.onsemi.com/pub/Collateral/PN2222-D.PDF" H 7040 3950 50  0001 L CNN
	1    7040 3950
	-1   0    0    -1  
$EndComp
Wire Wire Line
	6830 3580 7690 3580
$Comp
L Device:R R6
U 1 1 60A76B3E
P 7390 3950
F 0 "R6" V 7183 3950 50  0000 C CNN
F 1 "1K" V 7274 3950 50  0000 C CNN
F 2 "Resistor_THT:R_Axial_DIN0204_L3.6mm_D1.6mm_P5.08mm_Horizontal" V 7320 3950 50  0001 C CNN
F 3 "~" H 7390 3950 50  0001 C CNN
	1    7390 3950
	0    1    1    0   
$EndComp
$Comp
L Device:R R5
U 1 1 60A79287
P 6940 3260
F 0 "R5" H 7010 3306 50  0000 L CNN
F 1 "220R" H 7010 3215 50  0000 L CNN
F 2 "Resistor_THT:R_Axial_DIN0204_L3.6mm_D1.6mm_P5.08mm_Horizontal" V 6870 3260 50  0001 C CNN
F 3 "~" H 6940 3260 50  0001 C CNN
	1    6940 3260
	1    0    0    -1  
$EndComp
Wire Wire Line
	6940 3000 6940 3110
Wire Wire Line
	6940 3410 6940 3750
Wire Wire Line
	7540 3950 7610 3950
Wire Wire Line
	7610 3950 7610 3480
Wire Wire Line
	7610 3480 7690 3480
Wire Wire Line
	6940 4150 6940 4260
Wire Wire Line
	6940 4260 5930 4260
Connection ~ 5930 4260
Wire Wire Line
	8290 4180 8290 4260
Wire Wire Line
	8290 4260 6940 4260
Connection ~ 6940 4260
Text GLabel 8140 2980 0    50   Input ~ 0
+5V
Wire Wire Line
	8290 2980 8140 2980
$Comp
L Connector:Conn_01x05_Female J2
U 1 1 60A4074F
P 9690 3580
F 0 "J2" H 9490 4050 50  0000 L CNN
F 1 "HC-12 transceiver" H 9490 3930 50  0000 L CNN
F 2 "Connector_PinSocket_2.54mm:PinSocket_1x05_P2.54mm_Vertical" H 9690 3580 50  0001 C CNN
F 3 "~" H 9690 3580 50  0001 C CNN
	1    9690 3580
	1    0    0    -1  
$EndComp
Text Label 9750 3500 0    50   ~ 0
GND
Text Label 9750 3610 0    50   ~ 0
RX
Text Label 9750 3720 0    50   ~ 0
TX
Text Label 9750 3810 0    50   ~ 0
SET
Text GLabel 9360 3380 0    50   Input ~ 0
+5V
Wire Wire Line
	9360 3380 9490 3380
Wire Wire Line
	9490 3480 9380 3480
Wire Wire Line
	9380 3480 9380 4260
Connection ~ 8290 4260
Wire Wire Line
	8890 3580 9010 3580
Wire Wire Line
	9010 3580 9220 3680
Wire Wire Line
	9220 3680 9490 3680
Wire Wire Line
	8890 3680 9010 3680
Wire Wire Line
	9010 3680 9220 3580
Wire Wire Line
	9220 3580 9490 3580
Wire Wire Line
	9380 4260 8290 4260
Wire Wire Line
	3150 2700 2360 2700
$Comp
L Connector:Screw_Terminal_01x02 J1
U 1 1 60A72378
P 2160 2700
F 0 "J1" H 2180 2920 50  0000 C CNN
F 1 "Bus connector" H 1950 2830 50  0000 C CNN
F 2 "Connector_PinSocket_2.54mm:PinSocket_1x02_P2.54mm_Vertical" H 2160 2700 50  0001 C CNN
F 3 "~" H 2160 2700 50  0001 C CNN
	1    2160 2700
	-1   0    0    -1  
$EndComp
Wire Wire Line
	3150 3300 2360 3300
Wire Wire Line
	2360 3300 2360 2800
Text Label 2010 2720 0    50   ~ 0
L
Text Label 2010 2840 0    50   ~ 0
L
Text Label 9750 3400 0    50   ~ 0
Vcc
Wire Wire Line
	5030 3000 6940 3000
Wire Wire Line
	5930 3960 6230 3960
Wire Wire Line
	6230 3960 6230 3680
Connection ~ 5930 3960
Text GLabel 5540 3660 0    50   Input ~ 0
+5V
Wire Wire Line
	5930 3660 5710 3660
$Comp
L Device:CP C2
U 1 1 60AA1817
P 4360 4000
F 0 "C2" H 4478 4046 50  0000 L CNN
F 1 "220uF" H 4478 3955 50  0000 L CNN
F 2 "" H 4398 3850 50  0001 C CNN
F 3 "~" H 4360 4000 50  0001 C CNN
	1    4360 4000
	1    0    0    -1  
$EndComp
Connection ~ 5030 3000
Connection ~ 5030 4260
Wire Wire Line
	4360 3850 4360 3660
Connection ~ 4360 3660
Wire Wire Line
	4360 4150 4360 4260
Connection ~ 4360 4260
Wire Wire Line
	4360 4260 3980 4260
Wire Wire Line
	4280 3660 4360 3660
Connection ~ 5710 4260
Wire Wire Line
	5710 4260 5030 4260
Wire Wire Line
	4360 3660 4590 3660
Wire Wire Line
	4360 4260 5030 4260
Connection ~ 5710 3660
Wire Wire Line
	5710 3660 5540 3660
$EndSCHEMATC
