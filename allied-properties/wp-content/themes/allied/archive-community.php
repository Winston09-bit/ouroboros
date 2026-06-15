<?php
/**
 * Communities / Portfolio archive — project grid with region/status filter.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
get_header();
?>
<section class="page-banner">
	<div class="container">
		<p class="eyebrow" style="color:var(--color-accent);"><?php esc_html_e( 'Portfolio', 'allied' ); ?></p>
		<h1><?php esc_html_e( 'Communities', 'allied' ); ?></h1>
		<p class="lead"><?php esc_html_e( 'Residential communities we have acquired, entitled, and developed across our core markets.', 'allied' ); ?></p>
	</div>
</section>

<section class="section">
	<div class="container">
		<?php
		$regions = get_terms( array( 'taxonomy' => 'region', 'hide_empty' => true ) );
		if ( $regions && ! is_wp_error( $regions ) ) :
			?>
			<div class="filter-bar" role="tablist" aria-label="<?php esc_attr_e( 'Filter communities by region', 'allied' ); ?>">
				<a href="#" data-filter="all" class="is-active"><?php esc_html_e( 'All', 'allied' ); ?></a>
				<?php foreach ( $regions as $region ) : ?>
					<a href="#" data-filter="<?php echo esc_attr( $region->slug ); ?>"><?php echo esc_html( $region->name ); ?></a>
				<?php endforeach; ?>
			</div>
		<?php endif; ?>

		<?php if ( have_posts() ) : ?>
			<div class="grid grid--3">
				<?php while ( have_posts() ) : the_post(); ?>
					<?php get_template_part( 'template-parts/card-community' ); ?>
				<?php endwhile; ?>
			</div>
			<div class="section--tight"><?php the_posts_pagination( array( 'mid_size' => 1 ) ); ?></div>
		<?php else : ?>
			<p class="notice notice--info"><?php esc_html_e( 'No communities published yet. Add them under Communities in the WordPress admin.', 'allied' ); ?></p>
		<?php endif; ?>
	</div>
</section>

<?php get_template_part( 'template-parts/cta-band' ); ?>
<?php get_footer(); ?>
