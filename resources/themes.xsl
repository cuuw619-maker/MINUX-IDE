<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
  <xsl:output method="text" encoding="UTF-8"/>
  <xsl:template match="/themes">
    <xsl:text>{"default":"</xsl:text><xsl:value-of select="@default"/><xsl:text>","themes":[</xsl:text>
    <xsl:for-each select="theme">
      <xsl:if test="position() &gt; 1"><xsl:text>,</xsl:text></xsl:if>
      <xsl:text>{"id":"</xsl:text><xsl:value-of select="@id"/>
      <xsl:text>","name":"</xsl:text><xsl:value-of select="@name"/>
      <xsl:text>","accent":"</xsl:text><xsl:value-of select="@accent"/>
      <xsl:text>","accent_bg":"</xsl:text><xsl:value-of select="@accent-bg"/><xsl:text>"}</xsl:text>
    </xsl:for-each>
    <xsl:text>]}</xsl:text>
  </xsl:template>
</xsl:stylesheet>
