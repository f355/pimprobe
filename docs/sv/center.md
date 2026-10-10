# Centrum

Rutinerna probar motstående ytor och hittar deras mittpunkt.

<!-- guide-only -->
![Knappar och avstånd för centrummätning](../images/center.png)
<!-- /guide-only -->

## Välj mätning

- **Tapp / Hål:** rund utvändig / invändig geometri; mät X och Y.
- **Block / Ficka:** rektangulär utvändig / invändig geometri; mät X och Y.
- **X-ribba / X-spår:** upphöjd remsa / spår; mät bara tvärs över i X.
- **Y-ribba / Y-spår:** upphöjd remsa / spår; mät bara tvärs över i Y.
- **Z:** proba en yta nedåt och återgå till starthöjden i Z. **Djup** anger den största söksträckan.

X och Y anger riktningen **tvärs över** geometrin, inte dess längdriktning.

## Tapp, block eller ribba

Börja ungefär centrerat ovanför geometrin. Starthöjden i Z måste ge frigång över den vid passage.

**X/Y-söksträcka** är sträckan utåt från startläget till varje sida, inte geometrins bredd. Välj tillräcklig sträcka för att hela kulan ska hamna utanför båda kanterna. **Djup** är sänkningen från starthöjden i Z till höjden där sidan mäts, inklusive avståndet ovanför ämnet.

Proben sänks utanför varje sida och probar inåt. Den höjs till starthöjden i Z innan den passerar till andra sidan. Efter mätningen slutar den ovanför centrum på starthöjden i Z.

## Hål, ficka eller spår

Börja ungefär centrerat inne i öppningen på mäthöjden i Z. **X/Y-söksträcka** är den största sträckan från startläget mot endera väggen. Värdet 20 tillåter sökning till startkoordinaten −20 och +20, inte en total bredd på 20.

Proben passerar mellan väggarna på denna höjd, så vägen måste vara fri. **Djup** används inte. Proben slutar i uppmätt centrum utan att höjas i Z.

## Mått

Tvåaxliga rutiner mäter X först, kör till X-centrum och mäter sedan Y. För ribbor och spår flyttas bara den uppmätta axeln till centrum.

Resultaten visar bredd och längd för block/fickor, bredd för ribbor/spår och separata X/Y-mått för tappar/hål. Måtten inkluderar kompensation för kulradien. På runda geometrier kan en ocentrerad start ge ett kortare första X-mått; proba igen från centrum om du behöver en centrerad diametermätning.

<!-- guide-only -->
![Fickcentrum med bredd- och längdmått](../images/dimensions.png)

[Sätt arbetsnollpunkten från resultatet](results.md)
<!-- /guide-only -->
